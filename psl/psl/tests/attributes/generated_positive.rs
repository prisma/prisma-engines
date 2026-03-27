use psl::parser_database::ScalarType;

use crate::common::*;

#[test]
fn should_accept_generated_attribute_on_int_field() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
        }

        model Session {
          id             Int    @id
          statusPriority Int?   @generated("CASE status WHEN 'A' THEN 1 WHEN 'B' THEN 2 END")
        }
    "#};

    let schema = psl::parse_schema_without_extensions(dml).unwrap();
    let model = schema.assert_has_model("Session");

    model
        .assert_has_scalar_field("statusPriority")
        .assert_scalar_type(ScalarType::Int)
        .assert_is_generated_column();
}

#[test]
fn should_accept_generated_attribute_on_string_field() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
        }

        model User {
          id       Int     @id
          first    String
          last     String
          fullName String? @generated("first || ' ' || last")
        }
    "#};

    let schema = psl::parse_schema_without_extensions(dml).unwrap();
    let model = schema.assert_has_model("User");

    model
        .assert_has_scalar_field("fullName")
        .assert_scalar_type(ScalarType::String)
        .assert_is_generated_column();
}

#[test]
fn generated_field_should_be_readable() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
        }

        model Item {
          id       Int    @id
          price    Float
          tax      Float? @generated("price * 0.2")
        }
    "#};

    let schema = psl::parse_schema_without_extensions(dml).unwrap();
    let model = schema.assert_has_model("Item");

    // Generated fields should exist as scalar fields
    model
        .assert_has_scalar_field("tax")
        .assert_is_generated_column()
        .assert_optional();
}
