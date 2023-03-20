mod brin;
mod cockroachdb;
mod gin;
mod gist;
mod spgist;

use psl::parser_database::IndexAlgorithm;

use crate::{common::*, with_header, Provider};

#[test]
fn hash_index() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a], type: Hash)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Hash);
}

#[test]
fn hash_index_disallows_ops() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: Int4MinMaxOps)], type: Hash)
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int4MinMaxOps` is not supported with the `Hash` index type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int4MinMaxOps)], type: Hash)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn btree_index_disallows_ops() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: Int4MinMaxOps)], type: BTree)
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int4MinMaxOps` is not supported with the `BTree` index type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int4MinMaxOps)], type: BTree)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          id String @unique(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    assert_valid(&schema)
}

#[test]
fn compound_unique_sort_order() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(sort: Desc), b(sort: Asc)])
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    assert_valid(&schema)
}

#[test]
fn index_sort_order() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(sort: Desc)])
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    assert_valid(&schema)
}

#[test]
fn disallows_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @unique(length: 30) @test.VarChar(255)
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id String [1;91m@unique(length: 30)[0m @test.VarChar(255)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallows_compound_unique_length_prefix() {
    let dml = indoc! {r#"
        model A {
          a String
          b String
          @@unique([a(length: 10), b(length: 30)])
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  b String
        [1;94m14 | [0m  [1;91m@@unique([a(length: 10), b(length: 30)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallows_index_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a String

          @@index([a(length: 10)])
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The length argument is not supported in an index definition with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(length: 10)])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn operator_classes_not_allowed_with_unique() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  String
          b  String

          @@unique([a(ops: raw("foo")), b(ops: raw("bar"))])
        }
    "#};

    let dml = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": Operator classes can only be defined to fields in an @@index attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@unique([a(ops: raw("foo")), b(ops: raw("bar"))])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
