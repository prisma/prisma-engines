use crate::common::*;
use datamodel::ast::Span;
use datamodel::error::DatamodelError;

#[test]
fn nice_error_for_missing_model_keyword() {
    let dml = r#"
    User {
        id Int @id
    }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_validation_error(
        "This block is invalid. It does not start with any known Prisma schema keyword. Valid keywords include 'model', 'enum', 'datasource' and 'generator'.",
        Span::new(5, 36),
    ));
}
#[test]
fn nice_error_for_missing_model_keyword_2() {
    let dml = r#"
    model User {
        id Int @id
    }
    Todo {
        id
    }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_validation_error(
        "This block is invalid. It does not start with any known Prisma schema keyword. Valid keywords include 'model', 'enum', 'datasource' and 'generator'.",
        Span::new(47, 70),
    ));
}

#[test]
fn nice_error_on_incorrect_enum_field() {
    let dml = r#"
    enum Role {
        A-dmin
        User
    }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_validation_error(
        "The character `-` is not allowed in Enum Value names.",
        Span::new(25, 31),
    ));
}

#[test]
fn nice_error_missing_type() {
    let dml = r#"
    model User {
        id Int @id
        name
    }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_model_validation_error(
        "This field declaration is invalid. It is either missing a name or a type.",
        "User",
        Span::new(45, 50),
    ));
}

#[test]
fn nice_error_missing_attribute_name() {
    let dml = r#"
    model User {
        id Int @id @
    }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_validation_error(
        "The name of a Attribute must not be empty.",
        Span::new(38, 38),
    ));
}

#[test]
fn nice_error_missing_braces() {
    let dml = r#"
    model User
        id Int @id
    "#;

    let error = parse_error(dml);

    error.assert_length(2);
    error.assert_is_at(
        0,
        DatamodelError::new_validation_error(
            "This line is invalid. It does not start with any known Prisma schema keyword.",
            Span::new(5, 16),
        ),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_validation_error(
            "This line is invalid. It does not start with any known Prisma schema keyword.",
            Span::new(24, 35),
        ),
    );
}

#[test]
fn nice_error_broken_field_type_legacy_list() {
    let dml = r#"
    model User {
        id [Int] @id
    }"#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_legacy_parser_error(
        "To specify a list, please use `Type[]` instead of `[Type]`.",
        Span::new(29, 34),
    ));
}

#[test]
fn nice_error_broken_field_type_legacy_colon() {
    let dml = r#"
    model User {
        id: Int @id
    }"#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_legacy_parser_error(
        "Field declarations don't require a `:`.",
        Span::new(28, 29),
    ));
}

#[test]
fn nice_error_broken_field_type_legacy_required() {
    let dml = r#"
    model User {
        id Int! @id
    }"#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_legacy_parser_error(
        "Fields are required by default, `!` is no longer required.",
        Span::new(29, 33),
    ));
}

#[test]
fn nice_error_legacy_model_decl() {
    let dml = r#"
    type User {
        id Int @id
    }"#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_legacy_parser_error(
        "Model declarations have to be indicated with the `model` keyword.",
        Span::new(5, 9),
    ));
}

#[test]
fn nice_error_in_case_of_literal_type_in_env_var() {
    let source = r#"
    datasource ds {
      provider = "postgresql"
      url = env(DATABASE_URL)
    }
    "#;

    let error = parse_error_and_ignore_datasource_urls(source);

    error.assert_is(DatamodelError::new_type_mismatch_error(
        "String",
        "literal",
        "DATABASE_URL",
        Span::new(67, 79),
    ));
}

#[test]
fn nice_error_in_case_of_bool_type_in_env_var() {
    let source = r#"
    datasource ds {
      provider = "postgresql"
      url = env(true)
    }
    "#;

    let error = parse_error_and_ignore_datasource_urls(source);

    error.assert_is(DatamodelError::new_type_mismatch_error(
        "String",
        "boolean",
        "true",
        Span::new(67, 71),
    ));
}

#[test]
fn nice_error_in_case_of_numeric_type_in_env_var() {
    let source = r#"
    datasource ds {
      provider = "postgresql"
      url = env(4)
    }
    "#;

    let error = parse_error_and_ignore_datasource_urls(source);

    error.assert_is(DatamodelError::new_type_mismatch_error(
        "String",
        "numeric",
        "4",
        Span::new(67, 68),
    ));
}

#[test]
fn nice_error_in_case_of_array_type_in_env_var() {
    let source = r#"
    datasource ds {
      provider = "postgresql"
      url = env([DATABASE_URL])
    }
    "#;

    let error = parse_error_and_ignore_datasource_urls(source);

    error.assert_is(DatamodelError::new_type_mismatch_error(
        "String",
        "array",
        "(array)",
        Span::new(67, 81),
    ));
}

#[test]
fn optional_list_fields_must_error() {
    let dml = r#"
    model User {
        id Int @id
        names String[]?
    }"#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_legacy_parser_error(
        "Optional lists are not supported. Use either `Type[]` or `Type?`.",
        Span::new(51, 60),
    ));
}

#[test]
fn invalid_lines_at_the_top_level_must_render_nicely() {
    // https://github.com/prisma/vscode/issues/140
    // If a user types on the very last line we did not error nicely.
    // a new line fixed the problem but this is not nice.
    let dml = r#"model User {
        id Int @id
        names String
    }

    model Bl"#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_validation_error(
        "This line is invalid. It does not start with any known Prisma schema keyword.",
        Span::new(64, 72),
    ));
}

#[test]
fn invalid_lines_in_datasources_must_render_nicely() {
    let dml = r#"
    datasource mydb {
        provider = "postgres"
        url = "postgresql://localhost"
        this is an invalid line
    }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_validation_error(
        "This line is not a valid definition within a datasource.",
        Span::new(100, 124),
    ));
}

#[test]
fn invalid_lines_in_generators_must_render_nicely() {
    let dml = r#"
    generator js {
        provider = "js"
        this is an invalid line
    }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_validation_error(
        "This line is not a valid definition within a generator.",
        Span::new(52, 76),
    ));
}

#[test]
fn invalid_field_line_must_error_nicely() {
    // https://github.com/prisma/vscode/issues/140
    // If a user types on the very last line we did not error nicely.
    // a new line fixed the problem but this is not nice.
    let dml = r#"model User {
        id    Int @id
        foo   Bar Bla
    }"#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_validation_error(
        "This line is not a valid field or attribute definition.",
        Span::new(43, 57),
    ));
}
