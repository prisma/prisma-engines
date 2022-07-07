use crate::{common::*, with_header, Provider};

#[test]
fn on_mysql() {
    let dml = indoc! {r#"
        model A {
          id Int                    @id
          a  Unsupported("polygon")

          @@index([a(ops: raw("poly_ops"))], type: SpGist)
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given index type is not supported with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([a(ops: raw("poly_ops"))], [1;91mtype: SpGist[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

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

#[test]
fn only_single_column_allowed() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet
          b  String @test.Inet

          @@index([a, b], type: SpGist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": SpGist does not support multi-column indices.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@index([a, b], type: SpGist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
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
fn inet_ops_with_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Int

          @@index([a(ops: InetOps)], type: SpGist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetOps)], type: SpGist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn inet_ops_with_wrong_index_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a(ops: InetOps)], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetOps` is not supported with the `Gin` index type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetOps)], type: Gin)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// TextOps

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

#[test]
fn text_ops_with_wrong_prisma_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Int

          @@index([a(ops: TextOps)], type: SpGist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TextOps` points to the field `a` that is not of String type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TextOps)], type: SpGist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn no_ops_weird_type() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Int

          @@index([a], type: SpGist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The SpGist index type does not support the type of the field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a], type: SpGist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn text_ops_with_wrong_index_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Text

          @@index([a(ops: TextOps)], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `TextOps` is not supported with the `Gist` index type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: TextOps)], type: Gist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
