use crate::common::*;
use crate::types::helper::{
    test_native_types_compatibility, test_native_types_with_field_attribute_support,
    test_native_types_without_attributes,
};
use datamodel::{ast, diagnostics::DatamodelError};
use indoc::indoc;

const BLOB_TYPES: &[&str] = &["Blob", "LongBlob", "MediumBlob", "TinyBlob"];
const TEXT_TYPES: &[&str] = &["Text", "LongText", "MediumText", "TinyText"];

#[test]
fn text_and_blob_data_types_should_fail_on_index() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "You cannot define an index on fields with Native type {} of MySQL.",
            type_name
        )
    }

    for tpe in BLOB_TYPES {
        test_block_attribute_support(tpe, "Bytes", "index", &error_msg(tpe));
    }

    for tpe in TEXT_TYPES {
        test_block_attribute_support(tpe, "String", "index", &error_msg(tpe));
    }
}

#[test]
fn text_and_blob_data_types_can_not_be_unique() {
    fn error_msg(type_name: &str) -> String {
        format!("Native type {} cannot be unique in MySQL.", type_name)
    }

    for tpe in BLOB_TYPES {
        test_native_types_with_field_attribute_support(tpe, "Bytes", "unique", &error_msg(tpe), MYSQL_SOURCE);
        test_block_attribute_support(tpe, "Bytes", "unique", &error_msg(tpe));
    }

    for tpe in TEXT_TYPES {
        test_native_types_with_field_attribute_support(tpe, "String", "unique", &error_msg(tpe), MYSQL_SOURCE);
        test_block_attribute_support(tpe, "String", "unique", &error_msg(tpe));
    }
}

#[test]
fn text_and_blob_data_types_should_fail_on_id_attribute() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Native type {} of MySQL cannot be used on a field that is `@id` or `@@id`.",
            type_name
        )
    }

    for tpe in BLOB_TYPES {
        test_native_types_with_field_attribute_support(tpe, "Bytes", "id", &error_msg(tpe), MYSQL_SOURCE);
        test_block_attribute_support(tpe, "Bytes", "id", &error_msg(tpe));
    }

    for tpe in TEXT_TYPES {
        test_native_types_with_field_attribute_support(tpe, "String", "id", &error_msg(tpe), MYSQL_SOURCE);
        test_block_attribute_support(tpe, "String", "id", &error_msg(tpe));
    }
}

fn test_block_attribute_support(native_type: &str, scalar_type: &str, attribute_name: &str, error_msg: &str) {
    let id_field = if attribute_name == "id" {
        ""
    } else {
        "id     Int    @id"
    };
    let dml = format!(
        r#"
    model User {{
      {id_field}
      firstname {scalar_type} @db.{native_type}
      lastname  {scalar_type} @db.{native_type}
      @@{attribute_name}([firstname, lastname])
    }}
    "#,
        id_field = id_field,
        native_type = native_type,
        scalar_type = scalar_type,
        attribute_name = attribute_name
    );

    test_native_types_compatibility(&dml, error_msg, MYSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_bit_type() {
    for tpe in &["Bit(0)", "Bit(65)"] {
        let error_msg = format!(
            "Argument M is out of range for Native type {} of MySQL: M can range from 1 to 64.",
            &tpe
        );
        test_native_types_without_attributes(tpe, "Bytes", &error_msg, MYSQL_SOURCE);
    }
}

#[test]
fn should_only_allow_bit_one_for_booleans() {
    let expected_error =
        "Argument M is out of range for Native type Bit(2) of MySQL: only Bit(1) can be used as Boolean.";

    test_native_types_without_attributes("Bit(2)", "Boolean", expected_error, MYSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_char_type() {
    let error_msg = "Argument M is out of range for Native type Char(256) of MySQL: M can range from 0 to 255.";

    test_native_types_without_attributes("Char(256)", "String", error_msg, MYSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varchar_type() {
    let error_msg =
        "Argument M is out of range for Native type VarChar(655350) of MySQL: M can range from 0 to 65,535.";

    test_native_types_without_attributes("VarChar(655350)", "String", error_msg, MYSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_decimal_type() {
    fn error_msg(type_name: &str, arg: &str, range: &str) -> String {
        format!(
            "Argument M is out of range for Native type {} of MySQL: {} can range from {}.",
            type_name, arg, range
        )
    }

    let native_type = "Decimal(66,20)";

    test_native_types_without_attributes(
        native_type,
        "Decimal",
        &error_msg(native_type, "Precision", "1 to 65"),
        MYSQL_SOURCE,
    );

    let native_type = "Decimal(44,33)";

    test_native_types_without_attributes(
        native_type,
        "Decimal",
        &error_msg(native_type, "Scale", "0 to 30"),
        MYSQL_SOURCE,
    );
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = indoc!(
        r#"
        datasource db {
          provider = "mysql"
          url      = "mysql://"
        }

        model Blog {
            id     Int  @id
            dec Decimal @db.Decimal(2, 4)
        }
        "#
    );

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Decimal(2,4) native type in MySQL.",
        ast::Span::new(101, 131),
    ));
}

#[test]
fn should_fail_on_incompatible_scalar_type_with_tiny_int() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        model Blog {
            id     Int    @id
            bigInt DateTime @db.TinyInt
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type TinyInt is not compatible with declared field type DateTime, expected field type Boolean or Int.",
        ast::Span::new(172, 182),
    ));
}
