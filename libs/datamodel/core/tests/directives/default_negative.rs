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

#[test]
fn must_error_if_autoincrement_function_is_used_for_fields_that_are_not_int() {
    let dml = r#"
    model Model {
        id  Int    @id
        foo String @default(autoincrement())
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "The function `autoincrement()` can not be used on fields of type `String`.",
        "default",
        Span::new(70, 85),
    ));
}

#[test]
fn must_error_if_default_value_for_enum_is_not_valid() {
    let dml = r#"
    model Model {
        id Int @id
        enum A @default(B)
    }

    enum A {
        A
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "The defined default value is not a valid value of the enum specified for the field.",
        "default",
        Span::new(46, 65),
    ));
}

#[test]
fn must_error_if_using_non_id_auto_increment_on_sqlite() {
    let dml = r#"
    datasource db1 {
        provider = "sqlite"
        url = "file://test.db"
    }
    
    model Model {
        id      Int @id
        non_id  Int @default(autoincrement()) @unique
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "The `autoincrement()` default value is used on a non-id field even though the datasource does not support this.",
        "default",
        Span::new(142, 188),
    ));
}

#[test]
fn must_error_if_using_multiple_auto_increment_on_mysql() {
    let dml = r#"
    datasource db1 {
        provider = "mysql"
        url = "mysql://"
    }
    
    model Model {
        id      Int @id
        non_id  Int @default(autoincrement()) @unique
        non_id2  Int @default(autoincrement()) @unique
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "The `autoincrement()` default value is used multiple times on this model even though the underlying datasource only supports one instance per table.",
        "default",
        Span::new(89, 241),
    ));
}

#[test]
fn must_error_if_using_non_indexed_auto_increment_on_mysql() {
    let dml = r#"
    datasource db1 {
        provider = "mysql"
        url = "mysql://"
    }
    
    model Model {
        id      Int @id
        non_id  Int @default(autoincrement())
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_directive_validation_error(
        "The `autoincrement()` default value is used on a non-indexed field even though the datasource does not support this.",
        "default",
        Span::new(135, 173),
    ));
}
