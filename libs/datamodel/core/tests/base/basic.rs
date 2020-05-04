use crate::common::*;
use datamodel::ast::Span;
use datamodel::common::ScalarType;
use datamodel::error::DatamodelError;

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
    user_model.assert_is_embedded(false);
    user_model
        .assert_has_field("firstName")
        .assert_base_type(&ScalarType::String);
    user_model
        .assert_has_field("lastName")
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
    // The user model.
    model User {
        id Int @id
        // The first name.
        // Can be multi-line.
        firstName String
        lastName String
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_with_documentation("The user model.");
    user_model
        .assert_has_field("firstName")
        .assert_with_documentation("The first name.\nCan be multi-line.");
}

#[test]
fn must_error_for_invalid_model_names() {
    let dml = r#"
    model DateTimeFilter {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The model name `DateTimeFilter` is invalid. It is a reserved name. Please change it.",
        "DateTimeFilter",
        Span::new(5, 52),
    ));
}

#[test]
fn must_return_good_error_messages_for_numbers_in_enums() {
    let dml = r#"
    enum MyEnum {
        1
        TWO
        THREE
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_validation_error(
        "The name of a Enum Value must not start with a number.",
        Span::new(27, 28),
    ));
}

#[test]
fn invalid_line_must_not_break() {
    let dml = r#"
    $ /a/b/c:.
    
    model Blog {
      id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_validation_error(
        "This line is invalid. It does not start with any known Prisma schema keyword.",
        Span::new(5, 16),
    ));
}
