use psl::parser_database::IndexAlgorithm;

use crate::{common::*, with_header, Provider};

#[test]
fn array_field_default_ops() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Int[]

          @@index([a], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin);
}

#[test]
fn no_ops_json_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json

          @@index([a], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin);
}

#[test]
fn with_raw_unsupported() {
    let dml = indoc! {r#"
        model A {
          id Int                     @id
          a  Unsupported("geometry")

          @@index([a], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin);
}

#[test]
fn jsonb_column_as_the_last_in_index() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json
          b  Int[]

          @@index([b, a], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["b", "a"])
        .assert_type(IndexAlgorithm::Gin);
}

#[test]
fn jsonb_column_must_be_the_last_in_index() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json
          b  Int[]

          @@index([a, b], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Cockroach, &["cockroachDb"]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": A `Json` column is only allowed as the last column of an inverted index.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@index([a, b], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn custom_ops_not_supported() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json

          @@index([a(ops: JsonbOps)], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Cockroach, &["cockroachDb"]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": Custom operator classes are not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: JsonbOps)], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn raw_ops_not_supported() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json

          @@index([a(ops: raw("jsonb_ops"))], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Cockroach, &["cockroachDb"]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": Custom operator classes are not supported with the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: raw("jsonb_ops"))], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn wrong_field_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Cockroach, &["cockroachDb"]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The Gin index type does not support the type of the field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
