use crate::{common::*, types::helper::test_native_types_without_attributes};
use datamodel::{ast, diagnostics::DatamodelError};

#[test]
fn should_fail_on_invalid_precision_for_decimal_type() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for Native type {} of CockroachDB: Precision must be positive with a maximum value of 1000.",
            type_name
        )
    }

    let native_type = "Decimal(1001,3)";
    test_native_types_without_attributes(native_type, "Decimal", &error_msg(native_type), COCKROACHDB_SOURCE);
}

#[test]
fn should_fail_on_invalid_precision_for_time_types() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for Native type {} of CockroachDB: M can range from 0 to 6.",
            type_name
        )
    }

    for tpe in &["Timestamp", "Time"] {
        let native_type = &format!("{}(7)", tpe);
        test_native_types_without_attributes(native_type, "DateTime", &error_msg(native_type), COCKROACHDB_SOURCE);
        let native_type = &format!("{}(-1)", tpe);
        test_native_types_without_attributes(native_type, "DateTime", &error_msg(native_type), COCKROACHDB_SOURCE);
    }
}

#[test]
fn should_fail_on_argument_out_of_range_for_bit_data_types() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Argument M is out of range for Native type {} of CockroachDB: M must be a positive integer.",
            type_name
        )
    }

    for tpe in &["Bit", "VarBit"] {
        let native_type = &format!("{}(0)", tpe);
        test_native_types_without_attributes(native_type, "String", &error_msg(native_type), COCKROACHDB_SOURCE);
    }
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "cockroachdb"
          url      = "postgresql://"
        }

        generator js {
            provider        = "prisma-client-js"
            previewFeatures = ["Cockroachdb"]
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Decimal(2, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new(
        "The scale must not be larger than the precision for the Decimal(2,4) native type in CockroachDB.".into(),
        ast::Span::new(299, 329),
    ));
}
