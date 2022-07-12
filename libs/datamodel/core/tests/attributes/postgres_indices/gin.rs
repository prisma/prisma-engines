use crate::{common::*, with_header, Provider};

#[test]
fn on_mysql() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          a  Json

          @@index([a(ops: JsonbOps)], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given index type is not supported with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([a(ops: JsonbOps)], [1;91mtype: Gin[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn with_raw_unsupported() {
    let dml = indoc! {r#"
        model A {
          id Int                     @id
          a  Unsupported("tsvector")

          @@index([a(ops: raw("tsvector_ops"))], type: Gin)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::raw("tsvector_ops"));

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::JsonbOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::JsonbOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

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
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

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
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::JsonbPathOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::JsonbPathOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

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
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

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
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::ArrayOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
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
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

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
fn array_ops_invalid_index_type() {
    let dml = indoc! {r#"
        model A {
          id Int   @id
          a  Int[]

          @@index([a(ops: ArrayOps)], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `ArrayOps` is not supported with the `Gist` index type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: ArrayOps)], type: Gist)[0m
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

    let schema = with_header(dm, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("data");
    field.operator_class = Some(OperatorClass::raw("gin_trgm_ops"));

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_data_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gin),
        clustered: None,
    });
}
