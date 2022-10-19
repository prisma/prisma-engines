use crate::common::{expect, expect_error, indoc};

#[test]
fn should_fail_on_native_type_with_invalid_datasource_name() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
          id     Int    @id
          bigInt Int    @pg.Integer
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mThe prefix pg is invalid. It must be equal to the name of an existing datasource e.g. db. Did you mean to use db.Integer?[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int    @id
        [1;94m 8 | [0m  bigInt Int    [1;91m@pg.Integer[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expected);
}

#[test]
fn should_fail_on_native_type_with_invalid_number_of_arguments() {
    let dml = indoc! {r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
          id     Int    @id
          bigInt Int    @pg.Integer
          foobar String @pg.VarChar(2, 3, 4)
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type VarChar takes 1 optional arguments, but received 3.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  bigInt Int    @pg.Integer
        [1;94m 9 | [0m  foobar String [1;91m@pg.VarChar(2, 3, 4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expected);
}

#[test]
fn should_fail_on_native_type_with_unknown_type() {
    let dml = indoc! {r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
          id     Int    @id
          bigInt Int    @pg.Numerical(3, 4)
          foobar String @pg.VarChar(5)
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type Numerical is not supported for postgresql connector.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int    @id
        [1;94m 8 | [0m  bigInt Int    [1;91m@pg.Numerical(3, 4)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expected);
}

#[test]
fn should_fail_on_native_type_with_incompatible_type() {
    let dml = indoc! {r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
          id     Int    @id
          foobar Boolean @pg.VarChar(5)
          foo Int @pg.BigInt
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type VarChar is not compatible with declared field type Boolean, expected field type String.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int    @id
        [1;94m 8 | [0m  foobar Boolean [1;91m@pg.VarChar(5)[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNative type BigInt is not compatible with declared field type Int, expected field type BigInt.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  foobar Boolean @pg.VarChar(5)
        [1;94m 9 | [0m  foo Int [1;91m@pg.BigInt[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expected);
}

#[test]
fn should_fail_on_native_type_with_invalid_arguments() {
    let dml = indoc! {r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
          id     Int    @id
          foobar String @pg.VarChar(a)
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mExpected a nonnegative integer, but found (a).[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id     Int    @id
        [1;94m 8 | [0m  foobar String [1;91m@pg.VarChar(a)[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expected)
}

#[test]
fn should_fail_on_native_type_in_unsupported_postgres() {
    let dml = indoc! {r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
          id           Int                                           @id
          decimal      Unsupported("Decimal(10,2)")
          text         Unsupported("Text")
          unsupported  Unsupported("Some random stuff")
          unsupportes2 Unsupported("Some random (2,5) do something")
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: The type `Unsupported("Decimal(10,2)")` you specified in the type definition for the field `decimal` is supported as a native type by Prisma. Please use the native type notation `Decimal @pg.Decimal(10,2)` for full support.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id           Int                                           @id
        [1;94m 8 | [0m  [1;91mdecimal      Unsupported("Decimal(10,2)")[0m
        [1;94m 9 | [0m  text         Unsupported("Text")
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: The type `Unsupported("Text")` you specified in the type definition for the field `text` is supported as a native type by Prisma. Please use the native type notation `String @pg.Text` for full support.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  decimal      Unsupported("Decimal(10,2)")
        [1;94m 9 | [0m  [1;91mtext         Unsupported("Text")[0m
        [1;94m10 | [0m  unsupported  Unsupported("Some random stuff")
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expected);
}

#[test]
fn should_fail_on_native_type_in_unsupported_mysql() {
    let dml = indoc! {r#"
        datasource pg {
          provider = "mysql"
          url = "mysql://"
        }

        model Blog {
          id      Int                  @id
          text    Unsupported("Text")
          decimal Unsupported("Float")
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: The type `Unsupported("Text")` you specified in the type definition for the field `text` is supported as a native type by Prisma. Please use the native type notation `String @pg.Text` for full support.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id      Int                  @id
        [1;94m 8 | [0m  [1;91mtext    Unsupported("Text")[0m
        [1;94m 9 | [0m  decimal Unsupported("Float")
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: The type `Unsupported("Float")` you specified in the type definition for the field `decimal` is supported as a native type by Prisma. Please use the native type notation `Float @pg.Float` for full support.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  text    Unsupported("Text")
        [1;94m 9 | [0m  [1;91mdecimal Unsupported("Float")[0m
        [1;94m10 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expected);
}

#[test]
fn should_fail_on_native_type_in_unsupported_sqlserver() {
    let dml = indoc! {r#"
        datasource pg {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        model Blog {
          id      Int                 @id
          text    Unsupported("Text")
          decimal Unsupported("Real")
          TEXT    Unsupported("TEXT")
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: The type `Unsupported("Text")` you specified in the type definition for the field `text` is supported as a native type by Prisma. Please use the native type notation `String @pg.Text` for full support.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id      Int                 @id
        [1;94m 8 | [0m  [1;91mtext    Unsupported("Text")[0m
        [1;94m 9 | [0m  decimal Unsupported("Real")
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: The type `Unsupported("Real")` you specified in the type definition for the field `decimal` is supported as a native type by Prisma. Please use the native type notation `Float @pg.Real` for full support.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  text    Unsupported("Text")
        [1;94m 9 | [0m  [1;91mdecimal Unsupported("Real")[0m
        [1;94m10 | [0m  TEXT    Unsupported("TEXT")
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expected);
}
