use crate::common::*;
use datamodel::{ast, diagnostics::DatamodelError};

#[test]
fn should_fail_on_native_type_with_invalid_datasource_name() {
    let dml = r#"
        datasource db {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
            id     Int    @id
            bigInt Int    @pg.Integer
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new(
        "The prefix pg is invalid. It must be equal to the name of an existing datasource e.g. db. Did you mean to use db.Integer?".into(),
        ast::Span::new(178, 188),
    ));
}

#[test]
fn should_fail_on_native_type_with_invalid_number_of_arguments() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
            id     Int    @id
            bigInt Int    @pg.Integer
            foobar String @pg.VarChar(2, 3, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new(
        "Native type VarChar takes 1 optional arguments, but received 3.".into(),
        ast::Span::new(216, 235),
    ));
}

#[test]
fn should_fail_on_native_type_with_unknown_type() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
            id     Int    @id
            bigInt Int    @pg.Numerical(3, 4)
            foobar String @pg.VarChar(5)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new(
        "Native type Numerical is not supported for postgresql connector.".into(),
        ast::Span::new(178, 196),
    ));
}

#[test]
fn should_fail_on_native_type_with_incompatible_type() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
            id     Int    @id
            foobar Boolean @pg.VarChar(5)
            foo Int @pg.BigInt
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new(
            "Native type VarChar is not compatible with declared field type Boolean, expected field type String."
                .into(),
            ast::Span::new(179, 192),
        ),
    );

    error.assert_is_at(
        1,
        DatamodelError::new(
            "Native type BigInt is not compatible with declared field type Int, expected field type BigInt.".into(),
            ast::Span::new(214, 223),
        ),
    );
}

#[test]
fn should_fail_on_native_type_with_invalid_arguments() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
            id     Int    @id
            foobar String @pg.VarChar(a)
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mExpected a numeric value, but failed while parsing "a": invalid digit found in string.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m            id     Int    @id
        [1;94m 9 | [0m            foobar String @[1;91mpg.VarChar(a)[0m
        [1;94m   | [0m
    "#]];
    expect_error(dml, &expected)
}

#[test]
fn should_fail_on_native_type_in_unsupported_postgres() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
            id              Int    @id
            decimal         Unsupported("Decimal(10,2)")
            text            Unsupported("Text")
            unsupported     Unsupported("Some random stuff")
            unsupportes2    Unsupported("Some random (2,5) do something")
        }
    "#;

    let error = parse_error(dml);

    error.assert_are(&[
        DatamodelError::new_validation_error(
        "The type `Unsupported(\"Decimal(10,2)\")` you specified in the type definition for the field `decimal` is supported as a native type by Prisma. Please use the native type notation `Decimal @pg.Decimal(10,2)` for full support.".to_owned(),
        ast::Span::new(172, 217),
    ),
        DatamodelError::new_validation_error(
            "The type `Unsupported(\"Text\")` you specified in the type definition for the field `text` is supported as a native type by Prisma. Please use the native type notation `String @pg.Text` for full support.".to_owned(),
            ast::Span::new(229, 265),
        )
    ]);
}

#[test]
fn should_fail_on_native_type_in_unsupported_mysql() {
    let dml = r#"
        datasource pg {
          provider = "mysql"
          url = "mysql://"
        }

        model Blog {
            id          Int    @id
            text        Unsupported("Text")
            decimal     Unsupported("Float")
        }
    "#;

    let error = parse_error(dml);

    error.assert_are(&[
        DatamodelError::new_validation_error(
            "The type `Unsupported(\"Text\")` you specified in the type definition for the field `text` is supported as a native type by Prisma. Please use the native type notation `String @pg.Text` for full support.".to_owned(),
            ast::Span::new(160, 192),
        ),
        DatamodelError::new_validation_error(
            "The type `Unsupported(\"Float\")` you specified in the type definition for the field `decimal` is supported as a native type by Prisma. Please use the native type notation `Float @pg.Float` for full support.".to_owned(),
            ast::Span::new(204, 237),
        )
    ]);
}

#[test]
fn should_fail_on_native_type_in_unsupported_sqlserver() {
    let dml = r#"
        datasource pg {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        model Blog {
            id          Int    @id
            text        Unsupported("Text")
            decimal     Unsupported("Real")
            TEXT        Unsupported("TEXT")
        }
    "#;

    let error = parse_error(dml);

    error.assert_are(&[
        DatamodelError::new_validation_error(
            "The type `Unsupported(\"Text\")` you specified in the type definition for the field `text` is supported as a native type by Prisma. Please use the native type notation `String @pg.Text` for full support.".to_owned(),
            ast::Span::new(168, 200),
        ),
        DatamodelError::new_validation_error(
            "The type `Unsupported(\"Real\")` you specified in the type definition for the field `decimal` is supported as a native type by Prisma. Please use the native type notation `Float @pg.Real` for full support.".to_owned(),
            ast::Span::new(212, 244),
        )
    ]);
}
