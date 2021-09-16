use crate::common::*;
use datamodel::{dml, ScalarType};

#[test]
fn parse_scalar_types() {
    let dml = r#"
    model User {
        id           Int    @id
        firstName    String
        age          Int
        isPro        Boolean
        averageGrade Float
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_scalar_field("firstName")
        .assert_base_type(&ScalarType::String);
    user_model
        .assert_has_scalar_field("age")
        .assert_base_type(&ScalarType::Int);
    user_model
        .assert_has_scalar_field("isPro")
        .assert_base_type(&ScalarType::Boolean);
    user_model
        .assert_has_scalar_field("averageGrade")
        .assert_base_type(&ScalarType::Float);
}

#[test]
fn parse_field_arity() {
    let dml = r#"
    datasource mypg {
        provider = "postgres"
        url = "postgresql://asdlj"
    }

    model Post {
        id Int @id
        text String
        photo String?
        comments String[]
        enums    Enum[]
    }

    enum Enum {
        A
        B
        C
    }
    "#;

    let schema = parse(dml);
    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_scalar_field("text")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::Required);
    post_model
        .assert_has_scalar_field("photo")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::Optional);
    post_model
        .assert_has_scalar_field("comments")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::List);

    post_model
        .assert_has_scalar_field("enums")
        .assert_enum_type("Enum")
        .assert_arity(&dml::FieldArity::List);
}

#[test]
fn scalar_list_types_are_not_supported_by_default() {
    let dml = indoc! {r#"
        model Post {
          id         Int @id
          text       String
          photo      String?
          comments   String[]
          enums      Enum[]
          categories Category[] // make sure that relations still work
        }

        enum Enum {
          A
          B
          C
        }

        model Category {
          id   Int    @id
          name String
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "comments" in model "Post" can't be a list. The current connector does not support lists of primitive types.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m  photo      String?
        [1;94m 5 | [0m  [1;91mcomments   String[][0m
        [1;94m 6 | [0m  enums      Enum[]
        [1;94m   | [0m
        [1;91merror[0m: [1mField "enums" in model "Post" can't be a list. The current connector does not support lists of primitive types.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m  comments   String[]
        [1;94m 6 | [0m  [1;91menums      Enum[][0m
        [1;94m 7 | [0m  categories Category[] // make sure that relations still work
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn scalar_list_types_are_not_supported_by_mysql() {
    let dml = indoc! {r#"
        datasource mysql {
          provider = "mysql"
          url = "mysql://asdlj"
        }

        model Post {
          id Int @id
          text String
          photo String?
          comments String[]
          enums    Enum[]
        }

        enum Enum {
          A
          B
          C
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "comments" in model "Post" can't be a list. The current connector does not support lists of primitive types.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m  photo String?
        [1;94m10 | [0m  [1;91mcomments String[][0m
        [1;94m11 | [0m  enums    Enum[]
        [1;94m   | [0m
        [1;91merror[0m: [1mField "enums" in model "Post" can't be a list. The current connector does not support lists of primitive types.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m  comments String[]
        [1;94m11 | [0m  [1;91menums    Enum[][0m
        [1;94m12 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn json_type_must_work_for_some_connectors() {
    let dml = indoc! {r#"
        model User {
          id   Int    @id
          json Json
        }
    "#};

    // empty connector does support it
    parse(dml)
        .assert_has_model("User")
        .assert_has_scalar_field("json")
        .assert_base_type(&ScalarType::Json);

    let error = datamodel::parse_schema(&format!("{}\n{}", SQLITE_SOURCE, dml))
        .map(drop)
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating field `json` in model `User`: Field `json` in model `User` can't be of type Json. The current connector does not support the Json type.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  id   Int    @id
        [1;94m 9 | [0m  [1;91mjson Json[0m
        [1;94m10 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);

    // Postgres does support it
    parse(&format!("{}\n{}", POSTGRES_SOURCE, dml))
        .assert_has_model("User")
        .assert_has_scalar_field("json")
        .assert_base_type(&ScalarType::Json);

    // MySQL does support it
    parse(&format!("{}\n{}", MYSQL_SOURCE, dml))
        .assert_has_model("User")
        .assert_has_scalar_field("json")
        .assert_base_type(&ScalarType::Json);
}

#[test]
fn resolve_enum_field() {
    let dml = r#"
    model User {
        id Int @id
        email String
        role Role
    }

    enum Role {
        ADMIN
        USER
        PRO
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_scalar_field("email")
        .assert_base_type(&ScalarType::String);
    user_model.assert_has_scalar_field("role").assert_enum_type("Role");

    let role_enum = schema.assert_has_enum("Role");
    role_enum.assert_has_value("ADMIN");
    role_enum.assert_has_value("PRO");
    role_enum.assert_has_value("USER");
}
