use crate::common::*;
use datamodel::dml::*;
use prisma_value::PrismaValue;

#[test]
fn int_id_without_default_should_have_strategy_none() {
    let dml = r#"
    model Model {
        id Int @id
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_scalar_field("id").assert_is_id(user_model);
}

#[test]
fn int_id_with_default_autoincrement_should_have_strategy_auto() {
    let dml = r#"
    model Model {
        id Int @id @default(autoincrement())
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_scalar_field("id").assert_is_id(user_model);
}

#[test]
#[ignore] // bring back when we work on embeds
fn id_should_also_work_on_embedded_types() {
    let dml = r#"
    model Model {
        id Int @id

        @@embedded
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_scalar_field("id").assert_is_id(user_model);
}

#[test]
fn should_allow_string_ids_with_cuid() {
    let dml = r#"
    model Model {
        id String @id @default(cuid())
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::Expression(ValueGenerator::new_cuid()));
}

#[test]
fn should_allow_string_ids_with_uuid() {
    let dml = r#"
    model Model {
        id String @id @default(uuid())
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::Expression(ValueGenerator::new_uuid()));
}

#[test]
fn should_allow_string_ids_without_default() {
    let dml = r#"
    model Model {
        id String @id
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String);
}

#[test]
fn should_allow_string_ids_with_static_default() {
    let dml = r#"
    model Model {
        id String @id @default("")
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_default_value(DefaultValue::Single(PrismaValue::String(String::from(""))))
        .assert_base_type(&ScalarType::String);
}

#[test]
fn should_allow_int_ids_with_static_default() {
    let dml = r#"
    model Model {
        id Int @id @default(0)
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_default_value(DefaultValue::Single(PrismaValue::Int(0)))
        .assert_base_type(&ScalarType::Int);
}

#[test]
fn multi_field_ids_must_work() {
    let dml = r#"
    model Model {
        a String
        b Int
        @@id([a,b])
    }
    "#;

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
    let dml = r#"
    model Model {
        id Int @id @unique
    }
    "#;

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
        db_name: None,
        fields: vec!["id".into()],
        tpe: IndexType::Unique,
        defined_on_field: true,
    });
}
