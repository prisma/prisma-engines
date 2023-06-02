use crate::{common::*, with_header, Provider};
use psl::parser_database::ScalarType;

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

    let schema = psl::parse_schema(dml).unwrap();
    let user_model = schema.assert_has_model("User");

    user_model
        .assert_has_scalar_field("firstName")
        .assert_scalar_type(ScalarType::String);

    user_model
        .assert_has_scalar_field("age")
        .assert_scalar_type(ScalarType::Int);

    user_model
        .assert_has_scalar_field("isPro")
        .assert_scalar_type(ScalarType::Boolean);

    user_model
        .assert_has_scalar_field("averageGrade")
        .assert_scalar_type(ScalarType::Float);
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

    let schema = psl::parse_schema(dml).unwrap();
    let post_model = schema.assert_has_model("Post");

    post_model
        .assert_has_scalar_field("text")
        .assert_scalar_type(ScalarType::String)
        .assert_required();

    post_model
        .assert_has_scalar_field("photo")
        .assert_scalar_type(ScalarType::String)
        .assert_optional();

    post_model
        .assert_has_scalar_field("comments")
        .assert_scalar_type(ScalarType::String)
        .assert_list();
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
          postId Int
          post   Post @relation(fields: [postId], references: [id])
        }
    "#};

    let error = parse_unwrap_err(dml);

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

    let error = parse_unwrap_err(dml);

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
    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("json")
        .assert_scalar_type(ScalarType::Json);

    let error = parse_unwrap_err(&format!("{SQLITE_SOURCE}\n{dml}"));

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
    psl::parse_schema(format!("{POSTGRES_SOURCE}\n{dml}"))
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("json")
        .assert_scalar_type(ScalarType::Json);

    // MySQL does support it
    psl::parse_schema(format!("{MYSQL_SOURCE}\n{dml}"))
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("json")
        .assert_scalar_type(ScalarType::Json);
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

    let schema = psl::parse_schema(dml).unwrap();
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_scalar_field("email");

    let role_enum = schema.db.find_enum("Role").unwrap();
    let value_names: Vec<_> = role_enum.values().map(|v| v.name()).collect();
    assert_eq!(value_names, &["ADMIN", "USER", "PRO"]);
}

#[test]
fn json_list_type_must_work_for_some_connectors() {
    let dml = indoc! {r#"
        model User {
          id   Int    @id
          json_list Json[]
        }
    "#};

    let schema = with_header(dml, Provider::Cockroach, &["cockroachdb"]);

    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating field `json_list` in model `User`: Field `json_list` in model `User` can't be of type Json[]. The current connector does not support the Json List type.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id   Int    @id
        [1;94m13 | [0m  [1;91mjson_list Json[][0m
        [1;94m14 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);

    let schema = with_header(dml, Provider::Postgres, &[]);

    // Postgres does support it
    psl::parse_schema(schema)
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("json_list")
        .assert_scalar_type(ScalarType::Json);
}
