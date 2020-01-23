use crate::common::*;
use datamodel::dml::*;

#[test]
fn int_id_without_default_should_have_strategy_none() {
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
        .assert_id_strategy(IdStrategy::None);
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
        .assert_id_strategy(IdStrategy::None);
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
        .assert_default_value(DefaultValue::Expression(
            ValueGenerator::new("cuid".to_owned(), Vec::new()).unwrap(),
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
        .assert_default_value(DefaultValue::Expression(
            ValueGenerator::new("uuid".to_owned(), Vec::new()).unwrap(),
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
        .assert_default_value(DefaultValue::Single(ScalarValue::String(String::from(""))))
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
        .assert_id_strategy(IdStrategy::None)
        .assert_default_value(DefaultValue::Single(ScalarValue::Int(0)))
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
    user_model.assert_has_id_fields(&["a", "b"]);
}

#[test]
fn relation_field_as_id_must_work() {
    let dml = r#"
    model User {
        identification Identification @relation(references:[id]) @id
    }
    
    model Identification {
        id Int @id
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_field("identification").assert_is_id(true);
}

#[test]
fn relation_fields_as_part_of_compound_id_must_work() {
    let dml = r#"
    model User {
        name           String            
        identification Identification @relation(references:[id])

        @@id([name, identification])
    }
    
    model Identification {
        id Int @id
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_id_fields(&["name", "identification"]);
}
