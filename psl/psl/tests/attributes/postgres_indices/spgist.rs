use psl::parser_database::{IndexAlgorithm, OperatorClass};

use crate::{common::*, with_header, Provider};

#[test]
fn with_raw_unsupported() {
    let dml = indoc! {r#"
        model A {
          id Int                    @id
          a  Unsupported("polygon")

          @@index([a(ops: raw("poly_ops"))], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist)
        .assert_field("a")
        .assert_raw_ops("poly_ops");
}

#[test]
fn with_unsupported_no_ops() {
    let dml = indoc! {r#"
        model A {
          id Int                    @id
          a  Unsupported("polygon")

          @@index([a], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist);
}

// InetOps

#[test]
fn no_ops_inet_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist);
}

#[test]
fn inet_type_inet_ops() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a(ops: InetOps)], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist)
        .assert_field("a")
        .assert_ops(OperatorClass::InetOps);
}

#[test]
fn no_ops_char_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Char(255)

          @@index([a], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist);
}

#[test]
fn no_ops_varchar_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist);
}

#[test]
fn no_ops_text_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist);
}

#[test]
fn text_type_text_ops() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a(ops: TextOps)], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist)
        .assert_field("a")
        .assert_ops(OperatorClass::TextOps);
}

#[test]
fn no_native_type_text_ops() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: TextOps)], type: SpGist)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_type(IndexAlgorithm::SpGist)
        .assert_field("a")
        .assert_ops(OperatorClass::TextOps);
}
