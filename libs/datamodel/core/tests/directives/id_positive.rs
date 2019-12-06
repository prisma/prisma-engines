use crate::common::*;
use datamodel::dml::*;

// Ported from
// https://github.com/prisma/prisma/blob/master/server/servers/deploy/src/test/scala/com/prisma/deploy/migration/validation/IdDirectiveSpec.scala

#[test]
fn int_id_should_have_strategy_auto() {
    let dml = r#"
    model Model {
        id Int @id
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_field("id")
        .assert_is_id(true)
        .assert_id_sequence(None)
        .assert_id_strategy(IdStrategy::Auto);
}

#[test]
fn id_should_also_work_on_embedded_types() {
    let dml = r#"
    model Model {
        id Int @id

        @@embedded
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_field("id")
        .assert_is_id(true)
        .assert_id_sequence(None)
        .assert_id_strategy(IdStrategy::Auto);
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
        .assert_has_field("id")
        .assert_is_id(true)
        .assert_base_type(&ScalarType::String)
        .assert_id_strategy(IdStrategy::Auto)
        .assert_default_value(ScalarValue::Expression(
            String::from("cuid"),
            ScalarType::String,
            Vec::new(),
        ));
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
        .assert_has_field("id")
        .assert_is_id(true)
        .assert_id_strategy(IdStrategy::Auto)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(ScalarValue::Expression(
            String::from("uuid"),
            ScalarType::String,
            Vec::new(),
        ));
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
        .assert_has_field("id")
        .assert_is_id(true)
        .assert_id_strategy(IdStrategy::None)
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
        .assert_has_field("id")
        .assert_is_id(true)
        .assert_id_strategy(IdStrategy::None)
        .assert_default_value(ScalarValue::String(String::from("")))
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
        .assert_has_field("id")
        .assert_is_id(true)
        .assert_id_strategy(IdStrategy::Auto)
        .assert_default_value(ScalarValue::Int(0))
        .assert_base_type(&ScalarType::String);
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
    user_model.assert_has_id_fields(&["a", "b"]);
}
