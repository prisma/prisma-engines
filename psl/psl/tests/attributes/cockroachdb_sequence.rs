use crate::common::*;

#[test]
fn default_sequence_is_not_valid_on_postgres() {
    let schema = r#"
        datasource db {
            provider = "postgresql"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            id Int @id @default(sequence())
        }
    "#;

    let expected_error = expect![[r#"
        [1;91merror[0m: [1mUnknown function in @default(): `sequence` is not known. You can read about the available functions here: https://pris.ly/d/attribute-functions[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        model Test {
        [1;94m 8 | [0m            id Int @id @default([1;91msequence()[0m)
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected_error);
}

#[test]
fn default_sequence_is_valid_on_cockroachdb() {
    let schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            id Int @id @default(sequence())
        }
    "#;

    assert_valid(schema);
}

#[test]
fn default_sequence_with_one_argument_of_the_wrong_type_on_cockroachdb() {
    let schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            id Int @id @default(sequence(cache: true))
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mExpected a numeric value, but received literal value `true`.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        model Test {
        [1;94m 8 | [0m            id Int @id @default(sequence(cache: [1;91mtrue[0m))
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected);
}

#[test]
fn default_sequence_with_one_argument_is_valid_on_cockroachdb() {
    let schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            id Int @id @default(sequence(start: 12))
        }
    "#;

    assert_valid(schema);
}

#[test]
fn default_sequence_with_all_arguments_is_valid_on_cockroachdb() {
    let schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            id Int @id @default(sequence(virtual: true, cache: 10, increment: 3, minValue: 10, maxValue: 100, start: 12))
        }
    "#;

    assert_valid(schema);
}

#[test]
fn default_sequence_with_unknown_argument() {
    let schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            id Int @id @default(sequence(virtual: true, toppings: "cheese"))
        }
    "#;

    let expectation = expect![[r#"
        [1;91merror[0m: [1mUnexpected argument in `sequence()` function call[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        model Test {
        [1;94m 8 | [0m            id Int @id @default(sequence(virtual: true, [1;91mtoppings: "cheese"[0m))
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expectation);
}
