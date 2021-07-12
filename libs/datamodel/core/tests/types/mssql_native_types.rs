use crate::{
    common::*,
    types::helper::{
        test_native_types_compatibility, test_native_types_with_field_attribute_support,
        test_native_types_without_attributes,
    },
};
use datamodel::{ast, diagnostics::DatamodelError};
use indoc::indoc;
use native_types::{MsSqlType, MsSqlTypeParameter::*};

const BLOB_TYPES: &[&str] = &["VarBinary(Max)", "Image"];
const TEXT_TYPES: &[&str] = &["Text", "NText", "VarChar(Max)", "NVarChar(Max)", "Xml"];

#[test]
fn text_and_blob_data_types_should_fail_on_index() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "You cannot define an index on fields with Native type {} of SQL Server.",
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
        format!("Native type {} cannot be unique in SQL Server.", type_name)
    }

    for tpe in BLOB_TYPES {
        test_native_types_with_field_attribute_support(tpe, "Bytes", "unique", &error_msg(tpe), MSSQL_SOURCE);
        test_block_attribute_support(tpe, "Bytes", "unique", &error_msg(tpe));
    }

    for tpe in TEXT_TYPES {
        test_native_types_with_field_attribute_support(tpe, "String", "unique", &error_msg(tpe), MSSQL_SOURCE);
        test_block_attribute_support(tpe, "String", "unique", &error_msg(tpe));
    }
}

#[test]
fn text_and_blob_data_types_should_fail_on_id_attribute() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "Native type {} of SQL Server cannot be used on a field that is `@id` or `@@id`.",
            type_name
        )
    }

    for tpe in BLOB_TYPES {
        test_native_types_with_field_attribute_support(tpe, "Bytes", "id", &error_msg(tpe), MSSQL_SOURCE);
        test_block_attribute_support(tpe, "Bytes", "id", &error_msg(tpe));
    }

    for tpe in TEXT_TYPES {
        test_native_types_with_field_attribute_support(tpe, "String", "id", &error_msg(tpe), MSSQL_SOURCE);
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

    test_native_types_compatibility(&dml, error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = indoc!(
        r#"
        datasource db {
            provider = "sqlserver"
            url      = "sqlserver://"
        }

        model Blog {
            id  Int     @id
            dec Decimal @db.Decimal(2,4)
        }
    "#
    );

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Decimal(2,4) native type in SQL Server.",
        ast::Span::new(113, 142),
    ));
}

#[test]
fn should_fail_on_argument_out_of_range_for_char_type() {
    let error_msg =
        "Argument M is out of range for Native type Char(8001) of SQL Server: Length can range from 1 to 8000.";

    test_native_types_without_attributes("Char(8001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_nchar_type() {
    let error_msg =
        "Argument M is out of range for Native type NChar(4001) of SQL Server: Length can range from 1 to 4000.";

    test_native_types_without_attributes("NChar(4001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varchar_type() {
    let error_msg = "Argument M is out of range for Native type VarChar(8001) of SQL Server: Length can range from 1 to 8000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("VarChar(8001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_nvarchar_type() {
    let error_msg = "Argument M is out of range for Native type NVarChar(4001) of SQL Server: Length can range from 1 to 4000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("NVarChar(4001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varbinary_type() {
    let error_msg = "Argument M is out of range for Native type VarBinary(8001) of SQL Server: Length can range from 1 to 8000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("VarBinary(8001)", "Bytes", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_binary_type() {
    let error_msg =
        "Argument M is out of range for Native type Binary(8001) of SQL Server: Length can range from 1 to 8000.";

    test_native_types_without_attributes("Binary(8001)", "Bytes", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_incompatible_scalar_type_with_tiny_int() {
    let dml = r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        model Blog {
            id     Int    @id
            bigInt DateTime @db.Bit
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type Bit is not compatible with declared field type DateTime, expected field type Boolean or Int.",
        ast::Span::new(180, 186),
    ));
}

#[test]
fn should_fail_on_bad_type_params() {
    let dml = r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        model Blog {
            id     Int    @id
            s      String @db.NVarChar(Ma)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Invalid argument for type NVarChar: Ma. Allowed values: a number or `Max`.",
        ast::Span::new(178, 193),
    ));
}

#[test]
fn should_fail_on_too_many_type_params() {
    let dml = r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        model Blog {
            id     Int    @id
            s      String @db.NVarChar(1, 2)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type NVarChar takes 1 optional arguments, but received 2.",
        ast::Span::new(178, 195),
    ));
}

macro_rules! test_type {
    ($name:ident($(($input:expr, $output:expr)),+ $(,)?)) => {
            #[test]
            fn $name () {
                $(
                    let dml = format!(r#"
                        datasource db {{
                            provider = "sqlserver"
                            url = "sqlserver://"
                        }}

                        model Blog {{
                            id Int    @id
                            x  {}
                        }}
                    "#, $input);

                    let instance = parse(&dml)
                        .assert_has_model("Blog")
                        .assert_has_scalar_field("x")
                        .field_type
                        .native_type()
                        .unwrap()
                        .clone();

                    let result: MsSqlType = instance.deserialize_native_type();

                    assert_eq!($output, result);
                )+
            }
        }
}

mod test_type_mapping {
    use super::*;

    test_type!(tinyint(("Int @db.TinyInt", MsSqlType::TinyInt)));
    test_type!(smallint(("Int @db.SmallInt", MsSqlType::SmallInt)));
    test_type!(int(("Int @db.Int", MsSqlType::Int)));
    test_type!(money(("Float @db.Money", MsSqlType::Money)));
    test_type!(smallmoney(("Float @db.SmallMoney", MsSqlType::SmallMoney)));
    test_type!(real(("Float @db.Real", MsSqlType::Real)));
    test_type!(date(("DateTime @db.Date", MsSqlType::Date)));
    test_type!(time(("DateTime @db.Time", MsSqlType::Time)));
    test_type!(datetime(("DateTime @db.DateTime", MsSqlType::DateTime)));
    test_type!(datetime2(("DateTime @db.DateTime2", MsSqlType::DateTime2)));
    test_type!(text(("String @db.Text", MsSqlType::Text)));
    test_type!(ntext(("String @db.NText", MsSqlType::NText)));
    test_type!(image(("Bytes @db.Image", MsSqlType::Image)));
    test_type!(xml(("String @db.Xml", MsSqlType::Xml)));

    test_type!(datetimeoffset((
        "DateTime @db.DateTimeOffset",
        MsSqlType::DateTimeOffset
    )));

    test_type!(smalldatetime(("DateTime @db.SmallDateTime", MsSqlType::SmallDateTime)));

    test_type!(binary(
        ("Bytes @db.Binary", MsSqlType::Binary(None)),
        ("Bytes @db.Binary(4000)", MsSqlType::Binary(Some(4000)))
    ));

    test_type!(varbinary(
        ("Bytes @db.VarBinary", MsSqlType::VarBinary(None)),
        ("Bytes @db.VarBinary(4000)", MsSqlType::VarBinary(Some(Number(4000)))),
        ("Bytes @db.VarBinary(Max)", MsSqlType::VarBinary(Some(Max))),
    ));

    test_type!(char(
        ("String @db.Char", MsSqlType::Char(None)),
        ("String @db.Char(4000)", MsSqlType::Char(Some(4000)))
    ));

    test_type!(nchar(
        ("String @db.NChar", MsSqlType::NChar(None)),
        ("String @db.NChar(4000)", MsSqlType::NChar(Some(4000)))
    ));

    test_type!(varchar(
        ("String @db.VarChar", MsSqlType::VarChar(None)),
        ("String @db.VarChar(8000)", MsSqlType::VarChar(Some(Number(8000)))),
        ("String @db.VarChar(Max)", MsSqlType::VarChar(Some(Max))),
    ));

    test_type!(nvarchar(
        ("String @db.NVarChar", MsSqlType::NVarChar(None)),
        ("String @db.NVarChar(4000)", MsSqlType::NVarChar(Some(Number(4000)))),
        ("String @db.NVarChar(Max)", MsSqlType::NVarChar(Some(Max))),
    ));

    test_type!(boolean(
        ("Boolean @db.Bit", MsSqlType::Bit),
        ("Int @db.Bit", MsSqlType::Bit),
    ));

    test_type!(decimal(
        ("Decimal @db.Decimal", MsSqlType::Decimal(None)),
        ("Decimal @db.Decimal(32,16)", MsSqlType::Decimal(Some((32, 16)))),
    ));

    test_type!(number(
        ("Decimal @db.Decimal", MsSqlType::Decimal(None)),
        ("Decimal @db.Decimal(32,16)", MsSqlType::Decimal(Some((32, 16)))),
    ));

    test_type!(float(
        ("Float @db.Float", MsSqlType::Float(None)),
        ("Float @db.Float(24)", MsSqlType::Float(Some(24))),
        ("Float @db.Float(53)", MsSqlType::Float(Some(53))),
    ));
}
