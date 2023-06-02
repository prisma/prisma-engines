use psl::parser_database::{IndexAlgorithm, OperatorClass};

use crate::{common::*, with_header, Provider};

#[test]
fn with_raw_unsupported() {
    let dml = indoc! {r#"
        model A {
          id Int                     @id
          a  Unsupported("tsvector")

          @@index([a(ops: raw("tsvector_ops"))], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_raw_ops("tsvector_ops");
}

#[test]
fn with_unsupported_no_ops() {
    let dml = indoc! {r#"
        model A {
          id Int                     @id
          a  Unsupported("tsvector")

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

// Bit

#[test]
fn bit_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Bit

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn bit_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Bit

          @@index([a(ops: BitMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::BitMinMaxOps);
}

// VarBit

#[test]
fn varbit_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarBit

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn varbit_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarBit

          @@index([a(ops: VarBitMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::VarBitMinMaxOps);
}

// date

#[test]
fn date_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int       @id
          a  DateTime @test.Date

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn date_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Date

          @@index([a(ops: DateMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::DateMinMaxOps);
}

#[test]
fn date_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Date

          @@index([a(ops: DateMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::DateMinMaxMultiOps);
}

#[test]
fn date_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Date

          @@index([a(ops: DateBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::DateBloomOps);
}

// real

#[test]
fn real_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.Real

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn real_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.Real

          @@index([a(ops: Float4MinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float4MinMaxOps);
}

#[test]
fn real_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.DoublePrecision

          @@index([a(ops: Float4MinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Float4MinMaxOps` does not support native type `DoublePrecision` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Float4MinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn real_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.Real

          @@index([a(ops: Float4MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float4MinMaxMultiOps);
}

#[test]
fn real_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.DoublePrecision

          @@index([a(ops: Float4MinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Float4MinMaxMultiOps` does not support native type `DoublePrecision` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Float4MinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn real_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.Real

          @@index([a(ops: Float4BloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float4BloomOps);
}

#[test]
fn real_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.DoublePrecision

          @@index([a(ops: Float4BloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Float4BloomOps` does not support native type `DoublePrecision` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Float4BloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// double

#[test]
fn prisma_float_all_defaults() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn double_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.DoublePrecision

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn double_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.DoublePrecision

          @@index([a(ops: Float8MinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float8MinMaxOps);
}

#[test]
fn double_minmax_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float

          @@index([a(ops: Float8MinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float8MinMaxOps);
}

#[test]
fn double_minmaxmulti_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float

          @@index([a(ops: Float8MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float8MinMaxMultiOps);
}

#[test]
fn double_bloom_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float

          @@index([a(ops: Float8BloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float8BloomOps);
}

#[test]
fn double_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.Real

          @@index([a(ops: Float8MinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Float8MinMaxOps` does not support native type `Real` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Float8MinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn double_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.DoublePrecision

          @@index([a(ops: Float8MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float8MinMaxMultiOps);
}

#[test]
fn double_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.Real

          @@index([a(ops: Float8MinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Float8MinMaxMultiOps` does not support native type `Real` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Float8MinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn double_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.DoublePrecision

          @@index([a(ops: Float8BloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float8BloomOps);
}

#[test]
fn double_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.Real

          @@index([a(ops: Float8BloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Float8BloomOps` does not support native type `Real` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Float8BloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// inet

#[test]
fn inet_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn inet_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a(ops: InetMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::InetMinMaxOps);
}

#[test]
fn inet_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a(ops: InetMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetMinMaxOps` does not support native type `VarChar` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn inet_minmax_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: InetMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetMinMaxOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn inet_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a(ops: InetMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::InetMinMaxMultiOps);
}

#[test]
fn inet_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a(ops: InetMinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetMinMaxMultiOps` does not support native type `VarChar` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetMinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn inet_minmaxmulti_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: InetMinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetMinMaxMultiOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetMinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn inet_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a(ops: InetBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::InetBloomOps);
}

#[test]
fn inet_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a(ops: InetBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetBloomOps` does not support native type `VarChar` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn inet_bloom_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: InetBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetBloomOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn inet_inclusion_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a(ops: InetInclusionOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::InetInclusionOps);
}

#[test]
fn inet_inclusion_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a(ops: InetInclusionOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetInclusionOps` does not support native type `VarChar` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetInclusionOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn inet_inclusion_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: InetInclusionOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetInclusionOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetInclusionOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// int2

#[test]
fn int2_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn int2_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: Int2MinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int2MinMaxOps);
}

#[test]
fn int2_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Integer

          @@index([a(ops: Int2MinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int2MinMaxOps` does not support native type `Integer` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int2MinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn int2_minmax_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: Int2MinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int2MinMaxOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int2MinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn int2_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: Int2MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int2MinMaxMultiOps);
}

#[test]
fn int2_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Integer

          @@index([a(ops: Int2MinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int2MinMaxMultiOps` does not support native type `Integer` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int2MinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn int2_minmaxmulti_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: Int2MinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int2MinMaxMultiOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int2MinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn int2_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: Int2BloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int2BloomOps);
}

#[test]
fn int2_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Integer

          @@index([a(ops: Int2BloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int2BloomOps` does not support native type `Integer` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int2BloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn int2_bloom_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: Int2BloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int2BloomOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int2BloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// int4

#[test]
fn int4_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Integer

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn int4_default_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn int4_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Integer

          @@index([a(ops: Int4MinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int4MinMaxOps);
}

#[test]
fn int4_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: Int4MinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int4MinMaxOps` does not support native type `SmallInt` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int4MinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn int4_minmax_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: Int4MinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int4MinMaxOps);
}

#[test]
fn int4_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Integer

          @@index([a(ops: Int4MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int4MinMaxMultiOps);
}

#[test]
fn int4_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: Int4MinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int4MinMaxMultiOps` does not support native type `SmallInt` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int4MinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn int4_minmaxmulti_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: Int4MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int4MinMaxMultiOps);
}

#[test]
fn int4_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Integer

          @@index([a(ops: Int4BloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int4BloomOps);
}

#[test]
fn int4_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: Int4BloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `Int4BloomOps` does not support native type `SmallInt` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: Int4BloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn int4_bloom_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a(ops: Int4BloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int4BloomOps);
}

// int8

#[test]
fn int8_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  BigInt @test.BigInt

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn int8_default_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn int8_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  BigInt @test.BigInt

          @@index([a(ops: Int8MinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int8MinMaxOps);
}

#[test]
fn int8_minmax_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  BigInt

          @@index([a(ops: Int8MinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int8MinMaxOps);
}

#[test]
fn int8_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  BigInt @test.BigInt

          @@index([a(ops: Int8MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int8MinMaxMultiOps);
}

#[test]
fn int8_minmaxmulti_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  BigInt

          @@index([a(ops: Int8MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int8MinMaxMultiOps);
}

#[test]
fn int8_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  BigInt @test.BigInt

          @@index([a(ops: Int8BloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int8BloomOps);
}

#[test]
fn int8_bloom_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  BigInt

          @@index([a(ops: Int8BloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Int8BloomOps);
}

// numeric

#[test]
fn prisma_decimal_all_defaults() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Decimal

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn decimal_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Decimal @test.Decimal

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn decimal_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Decimal @test.Decimal

          @@index([a(ops: NumericMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::NumericMinMaxOps);
}

#[test]
fn decimal_minmax_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Int

          @@index([a(ops: NumericMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `NumericMinMaxOps` points to the field `a` that is not of Decimal type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: NumericMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn decimal_minmax_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Decimal

          @@index([a(ops: NumericMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::NumericMinMaxOps);
}

#[test]
fn decimal_minmaxmulti_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Int

          @@index([a(ops: NumericMinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `NumericMinMaxMultiOps` points to the field `a` that is not of Decimal type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: NumericMinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn decimal_minmaxmulti_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Decimal

          @@index([a(ops: NumericMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::NumericMinMaxMultiOps);
}

#[test]
fn decimal_bloom_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Int

          @@index([a(ops: NumericBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `NumericBloomOps` points to the field `a` that is not of Decimal type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: NumericBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn decimal_bloom_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Decimal

          @@index([a(ops: NumericBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::NumericBloomOps);
}

#[test]
fn decimal_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Float @test.DoublePrecision

          @@index([a(ops: Float8MinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::Float8MinMaxMultiOps);
}

#[test]
fn decimal_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Decimal @test.Decimal

          @@index([a(ops: NumericBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::NumericBloomOps);
}

// oid

#[test]
fn oid_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Oid

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn oid_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Oid

          @@index([a(ops: OidMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::OidMinMaxOps);
}

#[test]
fn oid_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: OidMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `OidMinMaxOps` does not support native type `SmallInt` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: OidMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn oid_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Oid

          @@index([a(ops: OidMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::OidMinMaxMultiOps);
}

#[test]
fn oid_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: OidMinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `OidMinMaxMultiOps` does not support native type `SmallInt` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: OidMinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn oid_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.Oid

          @@index([a(ops: OidBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::OidBloomOps);
}

#[test]
fn oid_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @test.SmallInt

          @@index([a(ops: OidBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `OidBloomOps` does not support native type `SmallInt` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: OidBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// char

#[test]
fn char_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Char

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn char_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Char

          @@index([a(ops: BpcharMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::BpcharMinMaxOps);
}

#[test]
fn char_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a(ops: BpcharMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `BpcharMinMaxOps` does not support native type `Text` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: BpcharMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn char_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Char

          @@index([a(ops: BpcharBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::BpcharBloomOps);
}

#[test]
fn char_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a(ops: BpcharBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `BpcharBloomOps` does not support native type `Text` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: BpcharBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// text

#[test]
fn prisma_text_all_defaults() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn text_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn varchar_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn text_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a(ops: TextMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TextMinMaxOps);
}

#[test]
fn varchar_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a(ops: TextMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TextMinMaxOps);
}

#[test]
fn text_minmax_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Int

          @@index([a(ops: TextMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TextMinMaxOps` points to the field `a` that is not of String type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TextMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn text_minmax_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: TextMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TextMinMaxOps);
}

#[test]
fn text_bloom_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int     @id
          a  Int

          @@index([a(ops: TextBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TextBloomOps` points to the field `a` that is not of String type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TextBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn text_bloom_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: TextBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TextBloomOps);
}

#[test]
fn text_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a(ops: TextBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TextBloomOps);
}

#[test]
fn varchar_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a(ops: TextBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TextBloomOps);
}

#[test]
fn no_native_type_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: TextBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TextBloomOps);
}

// timestamp

#[test]
fn prisma_datetime_all_defaults() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  DateTime

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn timestamp_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn timestamp_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimestampMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampMinMaxOps);
}

#[test]
fn timestamp_minmax_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime

          @@index([a(ops: TimestampMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampMinMaxOps);
}

#[test]
fn timestamp_minmaxmulti_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  DateTime

          @@index([a(ops: TimestampMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampMinMaxMultiOps);
}

#[test]
fn timestamp_bloom_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  DateTime

          @@index([a(ops: TimestampBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampBloomOps);
}

#[test]
fn timestamp_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Time

          @@index([a(ops: TimestampMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimestampMinMaxOps` does not support native type `Time` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimestampMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn timestamp_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimestampMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampMinMaxMultiOps);
}

#[test]
fn timestamp_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Time

          @@index([a(ops: TimestampMinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimestampMinMaxMultiOps` does not support native type `Time` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimestampMinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn timestamp_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimestampBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampBloomOps);
}

#[test]
fn timestamp_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Time

          @@index([a(ops: TimestampBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimestampBloomOps` does not support native type `Time` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimestampBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// timestamptz

#[test]
fn timestamptz_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamptz

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn timestamptz_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamptz

          @@index([a(ops: TimestampTzMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampTzMinMaxOps);
}

#[test]
fn timestamptz_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimestampTzMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimestampTzMinMaxOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimestampTzMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn timestamptz_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamptz

          @@index([a(ops: TimestampTzMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampTzMinMaxMultiOps);
}

#[test]
fn timestamptz_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimestampTzMinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimestampTzMinMaxMultiOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimestampTzMinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn timestamptz_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamptz

          @@index([a(ops: TimestampTzBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimestampTzBloomOps);
}

#[test]
fn timestamptz_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimestampTzBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimestampTzBloomOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimestampTzBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// time

#[test]
fn time_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Time

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn time_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Time

          @@index([a(ops: TimeMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimeMinMaxOps);
}

#[test]
fn time_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimeMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimeMinMaxOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimeMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn time_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Time

          @@index([a(ops: TimeMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimeMinMaxMultiOps);
}

#[test]
fn time_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimeMinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimeMinMaxMultiOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimeMinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn time_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Time

          @@index([a(ops: TimeBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimeBloomOps);
}

#[test]
fn time_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimeBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimeBloomOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimeBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// timetz

#[test]
fn timetz_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timetz

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn timetz_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timetz

          @@index([a(ops: TimeTzMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimeTzMinMaxOps);
}

#[test]
fn timetz_minmax_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimeTzMinMaxOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimeTzMinMaxOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimeTzMinMaxOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn timetz_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timetz

          @@index([a(ops: TimeTzMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimeTzMinMaxMultiOps);
}

#[test]
fn timetz_minmaxmulti_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimeTzMinMaxMultiOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimeTzMinMaxMultiOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimeTzMinMaxMultiOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn timetz_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timetz

          @@index([a(ops: TimeTzBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::TimeTzBloomOps);
}

#[test]
fn timetz_bloom_wrong_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int      @id
          a  DateTime @test.Timestamp

          @@index([a(ops: TimeTzBloomOps)], type: Brin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TimeTzBloomOps` does not support native type `Timestamp` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TimeTzBloomOps)], type: Brin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// uuid

#[test]
fn uuid_default_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Uuid

          @@index([a], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin);
}

#[test]
fn uuid_minmax_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Uuid

          @@index([a(ops: UuidMinMaxOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::UuidMinMaxOps);
}

#[test]
fn uuid_minmaxmulti_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Uuid

          @@index([a(ops: UuidMinMaxMultiOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::UuidMinMaxMultiOps);
}

#[test]
fn uuid_bloom_opclass() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Uuid

          @@index([a(ops: UuidBloomOps)], type: Brin)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::Brin)
        .assert_field("a")
        .assert_ops(OperatorClass::UuidBloomOps);
}
