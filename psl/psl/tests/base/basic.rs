use crate::common::*;
use psl::parser_database::ScalarType;

#[test]
fn parse_basic_model() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String
          lastName String
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let user_model = schema.assert_has_model("User");

    user_model
        .assert_has_scalar_field("firstName")
        .assert_scalar_type(ScalarType::String);

    user_model
        .assert_has_scalar_field("lastName")
        .assert_scalar_type(ScalarType::String);
}

#[test]
fn parse_basic_enum() {
    let dml = indoc! {r#"
        enum Roles {
          Admin
          User
          USER
          ADMIN
          ADMIN_USER
          Admin_User
          HHorse99
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let role_enum = schema.db.find_enum("Roles").unwrap();
    let values: Vec<_> = role_enum.values().map(|v| v.name()).collect();

    assert_eq!(
        values,
        &["Admin", "User", "USER", "ADMIN", "ADMIN_USER", "Admin_User", "HHorse99"]
    );
}

#[test]
fn parse_comments() {
    let dml = indoc! {r#"
        /// The user model.
        model User {
          id Int @id
          /// The first name.
          /// Can be multi-line.
          firstName String
          lastName String
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();

    let user_model = schema.assert_has_model("User");
    user_model.assert_with_documentation("The user model.");

    user_model
        .assert_has_scalar_field("firstName")
        .assert_with_documentation("The first name.\nCan be multi-line.");
}

#[test]
fn must_error_for_invalid_model_names() {
    let dml = indoc! {r#"
        model PrismaClient {
          id Int @id
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "PrismaClient": The model name `PrismaClient` is invalid. It is a reserved name. Please change it. Read more at https://pris.ly/d/naming-models[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel PrismaClient {[0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn must_error_for_invalid_enum_names() {
    let dml = indoc! {r#"
        enum PrismaClient {
          one
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r##"
        [1;91merror[0m: [1mError validating enum `PrismaClient`: The enum name `PrismaClient` is invalid. It is a reserved name. Please change it. Read more at https://www.prisma.io/docs/reference/tools-and-interfaces/prisma-schema/data-model#naming-enums[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91menum PrismaClient {[0m
        [1;94m 2 | [0m  one
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "##]];

    expectation.assert_eq(&error);
}

#[test]
fn must_return_good_error_messages_for_numbers_in_enums() {
    let dml = indoc! {r#"
        enum MyEnum {
          1
          TWO
          THREE
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: The name of a Enum Value must not start with a number.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0menum MyEnum {
        [1;94m 2 | [0m  [1;91m1[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn must_return_good_error_message_for_empty_enum() {
    let dml = indoc! {r#"
        enum MyEnum {

        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: An enum must have at least one value.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91menum MyEnum {[0m
        [1;94m 2 | [0m
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn must_return_good_error_message_for_enum_with_all_variants_commented_out() {
    let dml = indoc! {r#"
        enum MyEnum {
          // 1
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: An enum must have at least one value.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91menum MyEnum {[0m
        [1;94m 2 | [0m  // 1
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn invalid_line_must_not_break() {
    let dml = indoc! {r#"
        $ /a/b/c:.

        model Blog {
          id Int @id
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91m$ /a/b/c:.[0m
        [1;94m 2 | [0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn type_aliases_must_error() {
    let dml = indoc! {r#"
      type MyString = String @default("B")

      model A {
        id  Int      @id
        val MyString
      }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: Invalid type definition. Please check the documentation in https://pris.ly/d/composite-types[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mtype MyString = String @default("B")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn must_return_good_error_message_for_type_match() {
    let dml = indoc! {r#"
        model User {
          firstName String
        }
        model B {
          a  datetime
          b  footime
          c user
          d DB
          e JS
        }

        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext, pg_trgm]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mType "datetime" is neither a built-in type, nor refers to another model, composite type, or enum. Did you mean "DateTime"?[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0mmodel B {
        [1;94m 5 | [0m  a  [1;91mdatetime[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mType "footime" is neither a built-in type, nor refers to another model, composite type, or enum.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m  a  datetime
        [1;94m 6 | [0m  b  [1;91mfootime[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mType "user" is neither a built-in type, nor refers to another model, composite type, or enum. Did you mean "User"?[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  b  footime
        [1;94m 7 | [0m  c [1;91muser[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mType "DB" is neither a built-in type, nor refers to another model, composite type, or enum.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  c user
        [1;94m 8 | [0m  d [1;91mDB[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mType "JS" is neither a built-in type, nor refers to another model, composite type, or enum.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  d DB
        [1;94m 9 | [0m  e [1;91mJS[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}
