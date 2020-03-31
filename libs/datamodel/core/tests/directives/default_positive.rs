use crate::common::*;
use chrono::{DateTime, Utc};
use datamodel::{
    common::{ScalarType, ScalarValue},
    DefaultValue, ValueGenerator,
};

#[test]
fn should_set_default_for_all_scalar_types() {
    let dml = r#"
    model Model {
        id Int @id
        int Int @default(3)
        float Float @default(3.14)
        decimal Decimal @default(3.15)
        string String @default("String")
        boolean Boolean @default(false)
        dateTime DateTime @default("2019-06-17T14:20:57Z")
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_field("int")
        .assert_base_type(&ScalarType::Int)
        .assert_default_value(DefaultValue::Single(ScalarValue::Int(3)));
    user_model
        .assert_has_field("float")
        .assert_base_type(&ScalarType::Float)
        .assert_default_value(DefaultValue::Single(ScalarValue::Float(3.14)));
    user_model
        .assert_has_field("decimal")
        .assert_base_type(&ScalarType::Decimal)
        .assert_default_value(DefaultValue::Single(ScalarValue::Decimal(3.15)));
    user_model
        .assert_has_field("string")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::Single(ScalarValue::String(String::from("String"))));
    user_model
        .assert_has_field("boolean")
        .assert_base_type(&ScalarType::Boolean)
        .assert_default_value(DefaultValue::Single(ScalarValue::Boolean(false)));
    user_model
        .assert_has_field("dateTime")
        .assert_base_type(&ScalarType::DateTime)
        .assert_default_value(DefaultValue::Single(ScalarValue::DateTime(
            "2019-06-17T14:20:57Z".parse::<DateTime<Utc>>().unwrap(),
        )));
}

#[test]
fn should_set_default_an_enum_type() {
    let dml = r#"
    model Model {
        id Int @id
        role Role @default(A_VARIANT_WITH_UNDERSCORES)
    }

    enum Role {
        ADMIN
        MODERATOR
        A_VARIANT_WITH_UNDERSCORES
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::Single(ScalarValue::ConstantLiteral(String::from(
            "A_VARIANT_WITH_UNDERSCORES",
        ))));
}

#[test]
fn should_set_default_on_remapped_enum_type() {
    let dml = r#"
    model Model {
        id Int @id
        role Role @default(A_VARIANT_WITH_UNDERSCORES)
    }

    enum Role {
        ADMIN
        MODERATOR
        A_VARIANT_WITH_UNDERSCORES @map("A VARIANT WITH UNDERSCORES")
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::Single(ScalarValue::ConstantLiteral(String::from(
            "A_VARIANT_WITH_UNDERSCORES",
        ))));
}

#[test]
fn db_generated_function_must_work_for_enum_fields() {
    let dml = r#"
    model Model {
        id Int @id
        role Role @default(dbgenerated())
    }

    enum Role {
        ADMIN
        MODERATOR
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::Expression(ValueGenerator::new_dbgenerated()));
}
