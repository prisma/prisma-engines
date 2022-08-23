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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::raw("poly_ops"));

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::InetOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::TextOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::TextOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::SpGist),
        clustered: None,
    });
}
