use crate::{common::*, with_header, Provider};

#[test]
fn clustered_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int

          @@index([a], clustered: true)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_index_on_fields(&["a"])
        .assert_clustered(true);
}

#[test]
fn clustered_unique_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int @unique(clustered: true)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_unique_on_fields(&["a"])
        .assert_clustered(true);
}

#[test]
fn clustered_compound_unique_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int
          b  Int

          @@unique([a, b], clustered: true)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_unique_on_fields(&["a", "b"])
        .assert_clustered(true);
}

#[test]
fn non_clustered_id_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_id_on_fields(&["id"])
        .assert_clustered(false);
}

#[test]
fn non_clustered_compound_id_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          left  Int
          right Int

          @@id([left, right], clustered: false)
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::SqlServer, &[]))
        .unwrap()
        .assert_has_model("A")
        .assert_id_on_fields(&["left", "right"])
        .assert_clustered(false);
}
