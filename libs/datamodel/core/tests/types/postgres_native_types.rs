use crate::common::*;
use crate::types::helper::{test_native_types_with_field_attribute_support, test_native_types_without_attributes};
use datamodel::{ast, diagnostics::DatamodelError};

#[test]
fn should_fail_on_serial_data_types_with_number_default() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Sequential native type {} of Postgres must not have a static default value.",
            type_name
        )
    }

    for tpe in &["SmallSerial", "Serial", "BigSerial"] {
        test_native_types_with_field_attribute_support(tpe, "Int", "default(4)", &error_msg(tpe), POSTGRES_SOURCE);
    }
}

#[test]
fn should_fail_on_argument_out_of_range_for_bit_data_types() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for Native type {} of MySQL: M must be a positive integer.",
            type_name
        )
    }

    for tpe in &["Bit", "VarBit"] {
        test_native_types_without_attributes(&format!("{}(0)", tpe), "String", &error_msg(tpe), POSTGRES_SOURCE);
    }
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "postgres"
          url      = "postgresql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Decimal(2, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Decimal native type in Postgres.",
        ast::Span::new(289, 319),
    ));
}

#[test]
fn should_fail_on_native_type_numeric_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "postgres"
          url      = "postgresql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Numeric(2, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Numeric native type in Postgres.",
        ast::Span::new(289, 319),
    ));
}
