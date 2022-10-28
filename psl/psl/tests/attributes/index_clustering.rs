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

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![IndexField::new_in_model("a")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: Some(true),
    });
}

#[test]
fn clustered_unique_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int @unique(clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);
    let model = schema.assert_has_model("A");

    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_key".to_string()),
        fields: vec![IndexField::new_in_model("a")],
        tpe: IndexType::Unique,
        clustered: Some(true),
        defined_on_field: true,
        algorithm: None,
    });
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

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);
    let model = schema.assert_has_model("A");

    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_b_key".to_string()),
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Unique,
        clustered: Some(true),
        defined_on_field: false,
        algorithm: None,
    });
}

#[test]
fn non_clustered_id_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);
    let model = schema.assert_has_model("A");

    model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: Some("A_pkey".to_string()),
        fields: vec![PrimaryKeyField::new("id")],
        defined_on_field: true,
        clustered: Some(false),
    });
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

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);
    let model = schema.assert_has_model("A");

    model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: Some("A_pkey".to_string()),
        fields: vec![PrimaryKeyField::new("left"), PrimaryKeyField::new("right")],
        defined_on_field: false,
        clustered: Some(false),
    });
}
