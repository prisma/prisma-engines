use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError};

#[test]
fn fail_on_duplicate_models() {
    let dml = r#"
    model User {
        id Int @id
    }
    model User {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_duplicate_top_error(
        "User",
        "model",
        "model",
        Span::new(53, 57),
    ));
}

// From issue: https://github.com/prisma/prisma/issues/1988
#[test]
fn fail_on_duplicate_models_with_relations() {
    let dml = r#"
    model Post {
      id Int @id
    }

    model Post {
      id Int @id
      categories Categories[]
    }

    model Categories {
      post Post @relation(fields:[postId], references: [id])
      postId Int
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is_at(
        0,
        DatamodelError::new_duplicate_top_error("Post", "model", "model", Span::new(52, 56)),
    );
}

#[test]
fn fail_on_model_enum_conflict() {
    let dml = r#"
    enum User {
        Admin
        Moderator
    }
    model User {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_duplicate_top_error(
        "User",
        "model",
        "enum",
        Span::new(65, 69),
    ));
}
#[test]
fn fail_on_model_type_conflict() {
    let dml = r#"
    type User = String
    model User {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_duplicate_top_error(
        "User",
        "model",
        "type",
        Span::new(34, 38),
    ));
}

#[test]
fn fail_on_enum_type_conflict() {
    let dml = r#"
    type User = String
    enum User {
        Admin
        Moderator
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_duplicate_top_error(
        "User",
        "enum",
        "type",
        Span::new(33, 37),
    ));
}

#[test]
fn fail_on_duplicate_field() {
    let dml = r#"
    model User {
        id Int @id
        firstName String
        firstName String
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_duplicate_field_error(
        "User",
        "firstName",
        Span::new(70, 79),
    ));
}

#[test]
fn fail_on_duplicate_enum_value() {
    let dml = r#"
    enum Role {
        Admin
        Moderator
        Moderator
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_duplicate_enum_value_error(
        "Role",
        "Moderator",
        Span::new(57, 67),
    ));
}

#[test]
fn fail_on_reserved_name_for_enum() {
    let dml = r#"
    enum String {
        Admin
        Moderator
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_reserved_scalar_type_error(
        "String",
        Span::new(10, 16),
    ));
}

#[test]
fn fail_on_reserved_name_for_model() {
    let dml = r#"
    model DateTime {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_reserved_scalar_type_error(
        "DateTime",
        Span::new(11, 19),
    ));
}

#[test]
fn fail_on_reserved_name_fo_custom_type() {
    let dml = r#"
    type Int = String
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_reserved_scalar_type_error("Int", Span::new(10, 13)));
}
