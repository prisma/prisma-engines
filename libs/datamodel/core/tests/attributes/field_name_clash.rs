use crate::{with_header, Provider};
use expect_test::expect;
use indoc::indoc;

#[test]
fn naming_a_field_to_a_generated_id_name_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          a           Int
          b           Int
          a_b         Int

          @@id([a, b])
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The field `a_b` clashes with the `@@id` attribute. Please remove the name clash by providing a custom index name: @@id([..], name: "custom_name")[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91mmodel User {[0m
        [1;94m12 | [0m  a           Int
        [1;94m13 | [0m  b           Int
        [1;94m14 | [0m  a_b         Int
        [1;94m15 | [0m
        [1;94m16 | [0m  @@id([a, b])
        [1;94m17 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn naming_a_field_to_a_generated_uniq_name_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          a           Int
          b           Int
          a_b         Int

          @@unique([a, b])
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The field `a_b` clashes with the `@@unique` attribute. Please remove the name clash by providing a custom index name: @@unique([..], name: "custom_name")[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91mmodel User {[0m
        [1;94m12 | [0m  a           Int
        [1;94m13 | [0m  b           Int
        [1;94m14 | [0m  a_b         Int
        [1;94m15 | [0m
        [1;94m16 | [0m  @@unique([a, b])
        [1;94m17 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn naming_a_field_to_a_generated_index_name_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          a           Int
          b           Int
          a_b         Int
          uniq        Int @unique

          @@index([a, b])
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The field `a_b` clashes with the `@@index` attribute. Please remove the name clash by providing a custom index name: @@index([..], name: "custom_name")[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91mmodel User {[0m
        [1;94m12 | [0m  a           Int
        [1;94m13 | [0m  b           Int
        [1;94m14 | [0m  a_b         Int
        [1;94m15 | [0m  uniq        Int @unique
        [1;94m16 | [0m
        [1;94m17 | [0m  @@index([a, b])
        [1;94m18 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
