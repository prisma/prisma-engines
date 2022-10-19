use crate::common::*;

#[test]
fn must_error_on_list_default_value_for_singular() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = env("DATABASE_URL")
        }

        type Model {
          rel String @default(["hello"])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The default value of a non-list field cannot be a list.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mtype Model {
        [1;94m 7 | [0m  rel String [1;91m@default(["hello"])[0m
        [1;94m   | [0m
    "#]];
    expect_error(dml, &expectation);
}

#[test]
fn must_error_on_singular_default_value_for_list() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = env("DATABASE_URL")
        }

        type Model {
          rel String[] @default("hello")
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The default value of a list field must be a list.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mtype Model {
        [1;94m 7 | [0m  rel String[] [1;91m@default("hello")[0m
        [1;94m   | [0m
    "#]];
    expect_error(dml, &expectation);
}

#[test]
fn must_error_on_bad_value_inside_list_default() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = env("DATABASE_URL")
        }

        type Model {
          rel String[] @default(["hello", 101, "dalmatians"])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Expected a String value, but found `101`.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mtype Model {
        [1;94m 7 | [0m  rel String[] [1;91m@default(["hello", 101, "dalmatians"])[0m
        [1;94m   | [0m
    "#]];
    expect_error(dml, &expectation);
}

#[test]
fn must_error_if_default_value_type_mismatch() {
    let dml = indoc! {r#"
        type Composite {
          rel String @default(3)
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Expected a String value, but found `3`.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mtype Composite {
        [1;94m 2 | [0m  rel String [1;91m@default(3)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn datetime_defaults_must_be_valid_rfc3339() {
    let dml = indoc! {r#"
      datasource mongo {
        provider = "mongodb"
        url = "mongodb://"
      }

        type Composite {
          rel DateTime @default("Hugo")
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "Hugo" is not a valid rfc3339 datetime string. (input contains invalid characters)[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  type Composite {
        [1;94m 7 | [0m    rel DateTime @default([1;91m"Hugo"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_unknown_function_is_used() {
    let dml = indoc! {r#"
        type Composite {
          rel DateTime @default(unknown_function())
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mUnknown function in @default(): `unknown_function` is not known. You can read about the available functions here: https://pris.ly/d/attribute-functions[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mtype Composite {
        [1;94m 2 | [0m  rel DateTime @default([1;91munknown_function()[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_now_function_is_used_for_fields_that_are_not_datetime() {
    let dml = indoc! {r#"
        type Composite {
          foo String @default(now())
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The function `now()` cannot be used on fields of type `String`.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mtype Composite {
        [1;94m 2 | [0m  foo String [1;91m@default(now())[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_autoincrement_function_is_used() {
    let dml = indoc! {r#"
        type Composite {
          foo String @default(autoincrement())
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The function `autoincrement()` is not supported on composite fields.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mtype Composite {
        [1;94m 2 | [0m  foo String [1;91m@default(autoincrement())[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_default_value_for_enum_is_not_a_value() {
    let dml = indoc! {r#"
      type Composite {
        enum A @default(B)
      }

      enum A {
        A
      }
  "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The defined default value `B` is not a valid value of the enum specified for the field.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mtype Composite {
        [1;94m 2 | [0m  enum A [1;91m@default(B)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_default_value_for_enum_is_not_valid() {
    let dml = indoc! {r#"
      type Model {
        enum A @default(cuid())
      }

      enum A {
        A
      }
  "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Expected an enum value, but found `cuid()`.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mtype Model {
        [1;94m 2 | [0m  enum A [1;91m@default(cuid())[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_scalar_default_on_unsupported() {
    let dml = indoc! {r#"
        datasource db1 {
          provider = "postgresql"
          url = "postgresql://"
        }

        type Composite {
          balance Unsupported("some random stuff") @default(12)
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Composite field of type `Unsupported` cannot have default values.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mtype Composite {
        [1;94m 7 | [0m  balance Unsupported("some random stuff") [1;91m@default(12)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_values_are_not_allowed() {
    let dml = indoc! { r#"
        type A {
          id String @default(cuid(), map: "nope__nope__nope")
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The `map` argument is not allowed on a composite type field.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mtype A {
        [1;94m 2 | [0m  id String [1;91m@default(cuid(), map: "nope__nope__nope")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn default_on_composite_type_field_errors() {
    let schema = indoc! { r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
            street String
        }

        type User {
            address Address? @default("{ \"street\": \"broadway\"}")
        }
    "#};

    let error = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field `address` in composite type `Address`: Defaults on fields of type composite are not supported. Please remove the `@default` attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0mtype User {
        [1;94m11 | [0m    address Address? [1;91m@default("{ \"street\": \"broadway\"}")[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_on_dbgenerated_default() {
    let schema = r#"
        type User {
            nickname String @default(dbgenerated())
        }
    "#;

    let error = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Fields of composite types cannot have `dbgenerated()` as default.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m        type User {
        [1;94m 3 | [0m            nickname String [1;91m@default(dbgenerated())[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn json_defaults_must_be_valid_json() {
    let schema = r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        type Test {
            name Json @default("not json")
        }
    "#;

    let error = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "not json" is not a valid JSON string. (expected ident at line 1 column 2)[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        type Test {
        [1;94m 8 | [0m            name Json @default([1;91m"not json"[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn bytes_defaults_must_be_base64() {
    let schema = r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        type Test {
            name Bytes @default("not base64")
        }
    "#;

    let error = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "not base64" is not a valid base64 string. (Could not convert from `base64 encoded bytes` to `PrismaValue::Bytes`)[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        type Test {
        [1;94m 8 | [0m            name Bytes @default([1;91m"not base64"[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn int_defaults_must_not_contain_decimal_point() {
    let schema = r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        type Test {
            score Int @default(3.14)
        }
    "#;

    let error = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "3.14" is not a valid integer. (invalid digit found in string)[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        type Test {
        [1;94m 8 | [0m            score Int @default([1;91m3.14[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn bigint_defaults_must_not_contain_decimal_point() {
    let schema = r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        type Test {
            score BigInt @default(3.14)
        }
    "#;

    let error = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "3.14" is not a valid integer. (invalid digit found in string)[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        type Test {
        [1;94m 8 | [0m            score BigInt @default([1;91m3.14[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn boolean_defaults_must_be_true_or_false() {
    let schema = r#"
        type Test {
            isEdible Boolean @default(True)
        }
    "#;

    let error = parse_unwrap_err(schema);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": A boolean literal must be `true` or `false`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m        type Test {
        [1;94m 3 | [0m            isEdible Boolean @default([1;91mTrue[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn nested_scalar_list_defaults_are_disallowed() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = env("DBURL")
        }

        type Pizza {
            toppings String[] @default(["reblochon cheese", ["potato", "with", "rosmarin"], "onions"])
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Expected a String value, but found `["potato","with","rosmarin"]`.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        type Pizza {
        [1;94m 8 | [0m            toppings String[] [1;91m@default(["reblochon cheese", ["potato", "with", "rosmarin"], "onions"])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected);
}

#[test]
fn scalar_list_default_on_non_list_field() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = env("DBURL")
        }

        type Pizza {
            toppings String @default(["reblochon cheese", "potato", "rosmarin", "onions"])
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The default value of a non-list field cannot be a list.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        type Pizza {
        [1;94m 8 | [0m            toppings String [1;91m@default(["reblochon cheese", "potato", "rosmarin", "onions"])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected);
}

#[test]
fn dbgenerated_inside_scalar_list_default() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = env("DBURL")
        }

        type Pizza {
            toppings String[] @default(["reblochon cheese", dbgenerated("potato"), "rosmarin", "onions"])
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Expected a String value, but found `dbgenerated("potato")`.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        type Pizza {
        [1;94m 8 | [0m            toppings String[] [1;91m@default(["reblochon cheese", dbgenerated("potato"), "rosmarin", "onions"])[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected);
}
