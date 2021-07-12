use crate::common::*;
use datamodel::{ast::Span, diagnostics::DatamodelError};

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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
    ));
}

#[test]
fn must_error_if_default_value_for_list() {
    let dml = r#"
    datasource db {
        provider = "postgres"
        url = "postgres://"
    }

    model Model {
        id Int @id
        rel String[] @default(["hello"])
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on list field.",
        "default",
        Span::new(145, 163),
    ));
}

#[test]
fn must_error_if_default_value_type_mismatch() {
    let dml = r#"
    model Model {
        id Int @id
        rel String @default(3)
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Expected a String value, but received numeric value \"3\".",
        "default",
        Span::new(58, 68),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Expected a datetime value, but failed while parsing \"Hugo\": input contains invalid characters.",
        "default",
        Span::new(60, 75),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The function unknown_function is not a known function.",
        "default",
        Span::new(60, 87),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The function `now()` cannot be used on fields of type `String`.",
        "default",
        Span::new(62, 76),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The function `autoincrement()` cannot be used on fields of type `String`.",
        "default",
        Span::new(62, 86),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The defined default value is not a valid value of the enum specified for the field.",
        "default",
        Span::new(54, 64),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The `autoincrement()` default value is used on a non-id field even though the datasource does not support this.",
        "default",
        Span::new(138, 184),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The `autoincrement()` default value is used multiple times on this model even though the underlying datasource only supports one instance per table.",
        "default",
        Span::new(85, 237),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The `autoincrement()` default value is used on a non-indexed field even though the datasource does not support this.",
        "default",
        Span::new(131, 169),
    ));
}

#[test]
fn must_error_if_scalar_default_on_unsupported() {
    let dml = r#"
    datasource db1 {
        provider = "postgresql"
        url = "postgresql://"
    }

    model Model {
        id      Int @id
        balance Unsupported("some random stuff") @default(12)
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Expected a function value, but received numeric value \"12\".",
        "default",
        Span::new(183, 194),
    ));
}

#[test]
fn must_error_if_non_string_expression_in_function_default() {
    let dml = r#"
    model Model {
        id      Int @id
        balance Int @default(autoincrement(cuid()))
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Error validating: DefaultValue function parsing failed. The function arg should only be empty or a single String. Got: `[Function(\"cuid\", [], Span { start: 86, end: 92 })]`. You can read about the available functions here: https://pris.ly/d/attribute-functions",
        "default",
        Span::new(64, 94),
    ));
}

#[test]
fn must_error_if_non_string_expression_in_function_default_2() {
    let dml = r#"
    model Model {
        id      Int @id
        balance Int @default(dbgenerated(5))
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Error validating: DefaultValue function parsing failed. The function arg should only be empty or a single String. Got: `[NumericValue(\"5\", Span { start: 84, end: 85 })]`. You can read about the available functions here: https://pris.ly/d/attribute-functions",
        "default",
        Span::new(64, 87),
    ));
}

#[test]
fn must_error_on_empty_string_in_dbgenerated() {
    let dml = r#"
    model Model {
        id      Int @id
        balance Int @default(dbgenerated(""))
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "dbgenerated() takes either no argument, or a single nonempty string argument.",
        "default",
        Span::new(64, 88),
    ));
}

#[test]
fn dbgenerated_default_errors_must_not_cascade_into_other_errors() {
    let dml = r#"
    datasource ds {
        provider = "mysql"
        url = "mysql://"
    }

    model User {
        id        Int    @id
        role      Bytes
        role2     Bytes @ds.VarBinary(40) @default(dbgenerated(""))

        @@unique([role2, role])
    }
    "#;

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();
    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": dbgenerated() takes either no argument, or a single nonempty string argument.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m        role      Bytes
        [1;94m10 | [0m        role2     Bytes @ds.VarBinary(40) @[1;91mdefault(dbgenerated(""))[0m
        [1;94m   | [0m
    "#]];
    expectation.assert_eq(&error)
}
