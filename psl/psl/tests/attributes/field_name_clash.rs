use crate::{common::*, with_header, Provider};

#[test]
fn naming_a_scalar_field_same_as_generated_id_name_should_error() {
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

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The field `a_b` clashes with the `@@id` attribute's name. Please resolve the conflict by providing a custom id name: `@@id([...], name: "custom_name")`[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@id([a, b])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn naming_a_field_same_as_generated_uniq_name_should_error() {
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

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The field `a_b` clashes with the `@@unique` name. Please resolve the conflict by providing a custom id name: `@@unique([...], name: "custom_name")`[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@unique([a, b])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn naming_a_field_same_as_explicit_uniq_name_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          a           Int
          b           Int
          moo         Int

          @@unique([a, b], name: "moo")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The custom name `moo` specified for the `@@unique` attribute is already used as a name for a field. Please choose a different name.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91mmodel User {[0m
        [1;94m12 | [0m  a           Int
        [1;94m13 | [0m  b           Int
        [1;94m14 | [0m  moo         Int
        [1;94m15 | [0m
        [1;94m16 | [0m  @@unique([a, b], name: "moo")
        [1;94m17 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn naming_a_field_same_as_explicit_id_name_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          a           Int
          b           Int
          moo         Int

          @@id([a, b], name: "moo")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The custom name `moo` specified for the `@@id` attribute is already used as a name for a field. Please choose a different name.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91mmodel User {[0m
        [1;94m12 | [0m  a           Int
        [1;94m13 | [0m  b           Int
        [1;94m14 | [0m  moo         Int
        [1;94m15 | [0m
        [1;94m16 | [0m  @@id([a, b], name: "moo")
        [1;94m17 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
