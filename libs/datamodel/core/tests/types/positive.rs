use crate::common::*;
use datamodel::{
    common::{ScalarType, ScalarValue},
    DefaultValue, ValueGenerator,
};
use datamodel_connector::ScalarFieldType;

#[test]
fn should_apply_a_custom_type() {
    let dml = r#"
    type ID = String @id @default(cuid())

    model Model {
        id ID
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
fn should_recursively_apply_a_custom_type() {
    let dml = r#"
        type MyString = String
        type MyStringWithDefault = MyString @default(cuid())
        type ID = MyStringWithDefault @id

        model Model {
            id ID
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
fn should_be_able_to_handle_multiple_types() {
    let dml = r#"
    type ID = String @id @default(cuid())
    type UniqueString = String @unique
    type Cash = Int @default(0)

    model User {
        id       ID
        email    UniqueString
        balance  Cash
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("User");
    user_model
        .assert_has_field("id")
        .assert_is_id()
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::Expression(
            ValueGenerator::new("cuid".to_owned(), Vec::new()).unwrap(),
        ));

    user_model
        .assert_has_field("email")
        .assert_is_unique(true)
        .assert_base_type(&ScalarType::String);

    user_model
        .assert_has_field("balance")
        .assert_base_type(&ScalarType::Int)
        .assert_default_value(DefaultValue::Single(ScalarValue::Int(0)));
}

#[test]
fn should_be_able_to_define_custom_enum_types() {
    let dml = r#"
    type RoleWithDefault = Role @default(USER)

    model User {
        id Int @id
        role RoleWithDefault
    }

    enum Role {
        ADMIN
        USER
        CEO
    }
    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("User");

    user_model
        .assert_has_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::Single(ScalarValue::ConstantLiteral(String::from("USER"))));
}

#[test]
#[ignore]
fn should_handle_type_mappings() {
    let dml = r#"
        model Blog {
            id     Int    @id
            bigInt BigInt
        }
    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("Blog");

    user_model
        .assert_has_field("bigInt")
        .assert_connector_type(&ScalarFieldType::new("BigInt", ScalarType::Int, "bigint"));
}

#[test]
#[ignore]
fn should_handle_type_specifications() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
            id     Int @id
            bigInt Int @pg.BigInt
        }
    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("Blog");

    user_model
        .assert_has_field("bigInt")
        .assert_connector_type(&ScalarFieldType::new("BigInt", ScalarType::Int, "bigint"));
}
