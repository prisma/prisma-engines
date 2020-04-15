use crate::common::*;
use datamodel::ast::Span;
use datamodel::dml::*;
use datamodel::error::DatamodelError;
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
    user_model.assert_has_field("id").assert_is_id();
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
    user_model.assert_has_field("id").assert_is_id();
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
    user_model.assert_has_field("id").assert_is_id();
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
        .assert_is_id()
        .assert_base_type(&ScalarType::String)
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
        .assert_is_id()
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
        .assert_is_id()
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
        .assert_is_id()
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
        .assert_has_field("id")
        .assert_is_id()
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
    user_model.assert_has_id_fields(&["a", "b"]);
}

#[test]
fn relation_field_as_id_must_error() {
    let dml = r#"
    model User {
        identification Identification @relation(references:[id]) @id
    }
    
    model Identification {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_directive_validation_error(
        "The field `identification` is a relation field and cannot be marked with `@id`. Only scalar fields can be declared as id.",
        "id",
        Span::new(84, 86),
    ));
}

#[test]
fn relation_fields_as_part_of_compound_id_must_error() {
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

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The id definition refers to the relation fields identification. Id definitions must reference only scalar fields.",
        "User",
        Span::new(136, 162),
    ));
}
