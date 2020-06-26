use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError};

#[test]
fn must_error_if_default_value_for_relation_field() {
    let dml = r#"
    model Model {
        id Int @id
        rel A @default("")
    }

    model A {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
    ));
}

#[test]
fn must_error_if_default_value_for_list() {
    let dml = r#"
    model Model {
        id Int @id
        rel String[] @default(["hello"])
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "Cannot set a default value on list field.",
        "default",
        Span::new(60, 78),
    ));
}

#[test]
fn must_error_if_default_value_type_missmatch() {
    let dml = r#"
    model Model {
        id Int @id
        rel String @default(3)
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "Expected a String value, but received numeric value \"3\".",
        "default",
        Span::new(66, 67),
    ));
}

#[test]
fn must_error_if_default_value_parser_error() {
    let dml = r#"
    model Model {
        id Int @id
        rel DateTime @default("Hugo")
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "Expected a datetime value, but failed while parsing \"Hugo\": input contains invalid characters.",
        "default",
        Span::new(68, 74),
    ));
}

#[test]
fn must_error_if_unknown_function_is_used() {
    let dml = r#"
    model Model {
        id Int @id
        rel DateTime @default(unknown_function())
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "The function unknown_function is not a known function.",
        "default",
        Span::new(68, 86),
    ));
}

#[test]
fn must_error_if_now_function_is_used_for_fields_that_are_not_datetime() {
    let dml = r#"
    model Model {
        id  Int    @id
        foo String @default(now())
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "The function `now()` can not be used on fields of type `String`.",
        "default",
        Span::new(70, 75),
    ));
}
