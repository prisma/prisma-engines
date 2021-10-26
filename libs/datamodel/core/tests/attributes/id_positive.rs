use crate::attributes::with_postgres_provider;
use crate::common::*;
use datamodel::dml::*;
use prisma_value::PrismaValue;

#[test]
fn int_id_without_default_should_have_strategy_none() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_scalar_field("id").assert_is_id(user_model);
}

#[test]
fn int_id_with_default_autoincrement_should_have_strategy_auto() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @default(autoincrement())
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_scalar_field("id").assert_is_id(user_model);
}

#[test]
#[ignore] // bring back when we work on embeds
fn id_should_also_work_on_embedded_types() {
    let dml = indoc! {r#"
        model Model {
          id Int @id

          @@embedded
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_scalar_field("id").assert_is_id(user_model);
}

#[test]
fn should_allow_string_ids_with_cuid() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default(cuid())
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_cuid()));
}

#[test]
fn should_allow_string_ids_with_uuid() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default(uuid())
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_uuid()));
}

#[test]
fn should_allow_string_ids_without_default() {
    let dml = indoc! {r#"
        model Model {
          id String @id
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String);
}

#[test]
fn should_allow_string_ids_with_static_default() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default("")
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from(""))))
        .assert_base_type(&ScalarType::String);
}

#[test]
fn should_allow_int_ids_with_static_default() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @default(0)
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Int(0)))
        .assert_base_type(&ScalarType::Int);
}

#[test]
fn multi_field_ids_must_work() {
    let dml = indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b])
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: None,
        fields: vec!["a".into(), "b".into()],
        defined_on_field: false,
    });
}

#[test]
fn should_allow_unique_and_id_on_same_field() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @unique
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: None,
        fields: vec!["id".into()],
        defined_on_field: true,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_id_key".to_string()),
        fields: vec!["id".into()],
        tpe: IndexType::Unique,
        defined_on_field: true,
    });
}

#[test]
fn unnamed_and_unmapped_multi_field_ids_must_work() {
    let dml = with_postgres_provider(indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b])
        }
    "#});

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_id_fields(&["a", "b"]);
    user_model.assert_has_named_pk("Model_pkey");
}

#[test]
fn unmapped_singular_id_must_work() {
    let dml = with_postgres_provider(indoc! {r#"
        model Model {
          a String @id
        }
    "#});

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_id_fields(&["a"]);
    model.assert_has_named_pk("Model_pkey");
}

#[test]
fn named_multi_field_ids_must_work() {
    let dml = with_postgres_provider(indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], name: "compoundId")
        }
    "#});

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_id_fields(&["a", "b"]);
    user_model.assert_has_named_pk("Model_pkey");
}

#[test]
fn mapped_multi_field_ids_must_work() {
    let dml = with_postgres_provider(indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], map:"dbname")
        }
    "#});

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_id_fields(&["a", "b"]);
    user_model.assert_has_named_pk("dbname");
}

#[test]
fn mapped_singular_id_must_work() {
    let dml = with_postgres_provider(indoc! {r#"
        model Model {
          a String @id(map: "test")
        }

        model Model2 {
          a String @id(map: "test2")
        }
    "#});

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_id_fields(&["a"]);
    model.assert_has_named_pk("test");

    let model2 = datamodel.assert_has_model("Model2");
    model2.assert_has_id_fields(&["a"]);
    model2.assert_has_named_pk("test2");
}

#[test]
fn named_and_mapped_multi_field_ids_must_work() {
    let dml = with_postgres_provider(indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], name: "compoundId", map:"dbname")
        }
    "#});

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_id_fields(&["a", "b"]);
    user_model.assert_has_named_pk("dbname");
}
