use crate::common::*;
use datamodel::ScalarType;

#[test]
fn parse_basic_model() {
    let dml = r#"
    model User {
        id Int @id
        firstName String
        lastName String
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_scalar_field("firstName")
        .assert_base_type(&ScalarType::String);
    user_model
        .assert_has_scalar_field("lastName")
        .assert_base_type(&ScalarType::String);
}

#[test]
fn parse_basic_enum() {
    let dml = r#"
    enum Roles {
        Admin
        User
        USER
        ADMIN
        ADMIN_USER
        Admin_User
        HHorse99
    }
    "#;

    let schema = parse(dml);
    let role_enum = schema.assert_has_enum("Roles");
    role_enum.assert_has_value("ADMIN");
    role_enum.assert_has_value("USER");
    role_enum.assert_has_value("User");
    role_enum.assert_has_value("Admin");
    role_enum.assert_has_value("ADMIN_USER");
    role_enum.assert_has_value("Admin_User");
    role_enum.assert_has_value("HHorse99");
}

#[test]
fn parse_comments() {
    let dml = r#"
    /// The user model.
    model User {
        id Int @id
        /// The first name.
        /// Can be multi-line.
        firstName String
        lastName String
    }
    "#;

    let schema = parse(dml);
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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

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
