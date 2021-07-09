use crate::common::*;
use crate::types::helper::test_native_types_compatibility;
use crate::types::helper::test_native_types_without_attributes;
use datamodel::{ast, diagnostics::DatamodelError};
use native_types::PostgresType;

#[test]
fn xml_data_type_should_fail_on_index() {
    for attribute_name in &["index", "unique"] {
        let dml = format!(
            r#"
            model User {{
                id Int @id
                firstname String @db.Xml
                lastname  String @db.Xml
                @@{attribute_name}([firstname, lastname])
            }}
        "#,
            attribute_name = attribute_name
        );

        let error_msg = if *attribute_name == "index" {
            "You cannot define an index on fields with Native type Xml of Postgres."
        } else {
            "Native type Xml cannot be unique in Postgres."
        };

        test_native_types_compatibility(&dml, error_msg, POSTGRES_SOURCE);
    }
}

#[test]
fn should_fail_on_invalid_precision_for_decimal_type() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for Native type {} of Postgres: Precision must be positive with a maximum value of 1000.",
            type_name
        )
    }

    let native_type = "Decimal(1001,3)";
    test_native_types_without_attributes(native_type, "Decimal", &error_msg(native_type), POSTGRES_SOURCE);
}

#[test]
fn should_fail_on_invalid_precision_for_time_types() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for Native type {} of Postgres: M can range from 0 to 6.",
            type_name
        )
    }

    for tpe in &["Timestamp", "Time"] {
        let native_type = &format!("{}(7)", tpe);
        test_native_types_without_attributes(native_type, "DateTime", &error_msg(native_type), POSTGRES_SOURCE);
        let native_type = &format!("{}(-1)", tpe);
        test_native_types_without_attributes(native_type, "DateTime", &error_msg(native_type), POSTGRES_SOURCE);
    }
}

#[test]
fn should_fail_on_argument_out_of_range_for_bit_data_types() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for Native type {} of Postgres: M must be a positive integer.",
            type_name
        )
    }

    for tpe in &["Bit", "VarBit"] {
        let native_type = &format!("{}(0)", tpe);
        test_native_types_without_attributes(native_type, "String", &error_msg(native_type), POSTGRES_SOURCE);
    }
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "postgres"
          url      = "postgresql://"
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Decimal(2, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Decimal(2,4) native type in Postgres.",
        ast::Span::new(167, 197),
    ));
}

#[test]
fn xml_should_work_with_string_scalar_type() {
    let dml = format!(
        r#"
        {datasource}

        model Blog {{
            id  Int    @id
            dec String @db.Xml
        }}
    "#,
        datasource = POSTGRES_SOURCE
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Blog");
    let sft = user_model.assert_has_scalar_field("dec").assert_native_type();

    let postgres_tpe: PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_tpe, PostgresType::Xml);
}
