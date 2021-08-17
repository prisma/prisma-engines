use crate::common::*;
use datamodel::{ast::Span, diagnostics::DatamodelError};
use indoc::indoc;

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

#[test]
fn named_default_constraints_should_not_work_on_non_sql_server() {
    let dml = indoc! { r#"
        datasource test {
            provider = "postgres"
            url = "postgres://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["namedConstraints"]
        }

        model A {
            id Int @id @default(autoincrement())
            data String @default("beeb buub", map: "meow")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": You defined a database name for the default value of a field on the model. This is not supported by the provider.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id Int @id @default(autoincrement())
        [1;94m13 | [0m    data String @[1;91mdefault("beeb buub", map: "meow")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_are_not_allowed_on_identity() {
    let dml = indoc! { r#"
        datasource test {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["microsoftSqlServer", "namedConstraints"]
        }

        model A {
            id Int @id @default(autoincrement(), map: "nope__nope__nope")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Naming an autoincrement default value is not allowed.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m    id Int @id @[1;91mdefault(autoincrement(), map: "nope__nope__nope")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_cannot_have_duplicate_names() {
    let dml = indoc! { r#"
        datasource test {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["microsoftSqlServer", "namedConstraints"]
        }

        model A {
            id Int @id @default(autoincrement())
            a  String @default("asdf", map: "reserved")
        }

        model B {
            id Int @id @default(autoincrement())
            b  String @default("asdf", map: "reserved")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Given constraint name is already in use in the data model.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id Int @id @default(autoincrement())
        [1;94m13 | [0m    a  String @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@default": Given constraint name is already in use in the data model.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m    id Int @id @default(autoincrement())
        [1;94m18 | [0m    b  String @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_cannot_clash_with_pk_names() {
    let dml = indoc! { r#"
        datasource test {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["microsoftSqlServer", "namedConstraints"]
        }

        model A {
            id Int @id @default(autoincrement())
            a  String @default("asdf", map: "reserved")
        }

        model B {
            id Int @id(map: "reserved") @default(autoincrement())
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Given constraint name is already in use in the data model.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id Int @id @default(autoincrement())
        [1;94m13 | [0m    a  String @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@id": Given constraint name is already in use in the data model.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0mmodel B {
        [1;94m17 | [0m    id Int @[1;91mid(map: "reserved")[0m @default(autoincrement())
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_cannot_clash_with_fk_names() {
    let dml = indoc! { r#"
        datasource test {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["microsoftSqlServer", "namedConstraints"]
        }

        model A {
            id  Int @id @default(autoincrement())
            a   String  @default("asdf", map: "reserved")
            b   B       @relation(fields: [bId], references: [id], map: "reserved")
            bId Int
        }

        model B {
            id Int @id @default(autoincrement())
            as A[]
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Given constraint name is already in use in the data model.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id  Int @id @default(autoincrement())
        [1;94m13 | [0m    a   String  @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Given constraint name is already in use in the data model.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    a   String  @default("asdf", map: "reserved")
        [1;94m14 | [0m    b   B       @relation(fields: [bId], references: [id], [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
