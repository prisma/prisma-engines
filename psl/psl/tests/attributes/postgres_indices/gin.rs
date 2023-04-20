use psl::parser_database::{IndexAlgorithm, OperatorClass};

use crate::{common::*, with_header, Provider};

#[test]
fn with_raw_unsupported() {
    let dml = indoc! {r#"
        model A {
          id Int                     @id
          a  Unsupported("tsvector")

          @@index([a(ops: raw("tsvector_ops"))], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin)
        .assert_field("a")
        .assert_raw_ops("tsvector_ops");
}

#[test]
fn with_unsupported_no_ops() {
    let dml = indoc! {r#"
        model A {
          id Int                     @id
          a  Unsupported("tsvector")

          @@index([a], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin);
}

// JsonbOps

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
fn no_ops_jsonb_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json @test.JsonB

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
fn valid_jsonb_ops_with_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json @test.JsonB

          @@index([a(ops: JsonbOps)], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin)
        .assert_field("a")
        .assert_ops(OperatorClass::JsonbOps);
}

#[test]
fn valid_jsonb_ops_without_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json

          @@index([a(ops: JsonbOps)], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin)
        .assert_field("a")
        .assert_ops(OperatorClass::JsonbOps);
}

#[test]
fn jsonb_ops_with_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Int

          @@index([a(ops: JsonbOps)], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `JsonbOps` points to the field `a` that is not of Json type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: JsonbOps)], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn jsonb_ops_invalid_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json @test.Json

          @@index([a(ops: JsonbOps)], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `JsonbOps` does not support native type `Json` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: JsonbOps)], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn jsonb_ops_invalid_index_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json @test.JsonB

          @@index([a(ops: JsonbOps)], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `JsonbOps` is not supported with the `Gist` index type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: JsonbOps)], type: Gist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// JsonbPathOps

#[test]
fn valid_jsonb_path_ops_with_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json @test.JsonB

          @@index([a(ops: JsonbPathOps)], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin)
        .assert_field("a")
        .assert_ops(OperatorClass::JsonbPathOps);
}

#[test]
fn valid_jsonb_path_ops_without_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json

          @@index([a(ops: JsonbPathOps)], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin)
        .assert_field("a")
        .assert_ops(OperatorClass::JsonbPathOps);
}

#[test]
fn jsonb_path_ops_invalid_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json @test.Json

          @@index([a(ops: JsonbPathOps)], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `JsonbPathOps` does not support native type `Json` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: JsonbPathOps)], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn jsonb_path_ops_with_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Int

          @@index([a(ops: JsonbPathOps)], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `JsonbPathOps` points to the field `a` that is not of Json type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: JsonbPathOps)], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn jsonb_path_ops_invalid_index_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json @test.JsonB

          @@index([a(ops: JsonbPathOps)], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `JsonbPathOps` is not supported with the `Gist` index type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: JsonbPathOps)], type: Gist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// ArrayOps

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
fn array_field_array_ops() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Int[]

          @@index([a(ops: ArrayOps)], type: Gin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Gin)
        .assert_field("a")
        .assert_ops(OperatorClass::ArrayOps);
}

#[test]
fn non_array_field_array_ops() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: ArrayOps)], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `ArrayOps` expects the type of field `a` to be an array.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: ArrayOps)], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn gin_raw_ops_to_supported_type() {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @test.VarChar

          @@index([data(ops: raw("gin_trgm_ops"))], type: Gin)
        }
    "#;

    psl::parse_schema(with_header(dm, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["data"])
        .assert_type(IndexAlgorithm::Gin)
        .assert_field("data")
        .assert_raw_ops("gin_trgm_ops");
}
