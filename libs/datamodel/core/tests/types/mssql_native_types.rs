use std::convert::TryFrom;

use crate::common::*;
use crate::types::helper::{
    test_native_types_compatibility, test_native_types_with_field_attribute_support,
    test_native_types_without_attributes,
};
use datamodel::{ast, diagnostics::DatamodelError};
use indoc::indoc;
use native_types::MsSqlType;
use native_types::TypeParameter;

const BLOB_TYPES: &[&'static str] = &["VarBinary(Max)", "Image"];
const TEXT_TYPES: &[&'static str] = &["Text", "NText", "VarChar(Max)", "NVarChar(Max)"];

#[test]
fn text_and_blob_data_types_should_fail_on_index() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "You can not define an index on fields with Native type {} of SQL Server.",
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
        format!("Native type {} can not be unique in SQL Server.", type_name)
    }

    for tpe in BLOB_TYPES {
        test_native_types_with_field_attribute_support(tpe, "Bytes", "unique", &error_msg(tpe), MSSQL_SOURCE);
        test_block_attribute_support(tpe, "Bytes", "unique", dbg!(&error_msg(tpe)));
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
            "Native type {} of SQL Server can not be used on a field that is `@id` or `@@id`.",
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

    test_native_types_compatibility(&dml, &error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = indoc!(
        r#"
        datasource db {
            provider = "sqlserver"
            url      = "sqlserver://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id  Int     @id
            dec Decimal @db.Decimal(2, 4)
        }
    "#
    );

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Decimal native type in SQL Server.",
        ast::Span::new(203, 233),
    ));
}

#[test]
fn should_fail_on_native_type_numeric_when_scale_is_bigger_than_precision() {
    let dml = indoc!(
        r#"
        datasource db {
            provider = "sqlserver"
            url      = "sqlserver://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id  Int     @id
            dec Decimal @db.Numeric(2, 4)
        }
    "#
    );

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Numeric native type in SQL Server.",
        ast::Span::new(203, 233),
    ));
}

#[test]
fn should_fail_on_argument_out_of_range_for_char_type() {
    let error_msg = "Argument M is out of range for Native type Char of SQL Server: Length can range from 1 to 4000.";

    test_native_types_without_attributes("Char(4001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_nchar_type() {
    let error_msg = "Argument M is out of range for Native type NChar of SQL Server: Length can range from 1 to 2000.";

    test_native_types_without_attributes("NChar(2001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varchar_type() {
    let error_msg = "Argument M is out of range for Native type VarChar of SQL Server: Length can range from 1 to 4000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("VarChar(4001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_nvarchar_type() {
    let error_msg = "Argument M is out of range for Native type NVarChar of SQL Server: Length can range from 1 to 2000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("NVarChar(2001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varbinary_type() {
    let error_msg = "Argument M is out of range for Native type VarBinary of SQL Server: Length can range from 1 to 4000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("VarBinary(4001)", "Bytes", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_binary_type() {
    let error_msg = "Argument M is out of range for Native type Binary of SQL Server: Length can range from 1 to 4000.";

    test_native_types_without_attributes("Binary(4001)", "Bytes", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_incompatible_scalar_type_with_tiny_int() {
    let dml = r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt DateTime @db.Bit
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type Bit is not compatible with declared field type DateTime, expected field type Boolean or Int.",
        ast::Span::new(302, 308),
    ));
}

macro_rules! test_type {
    ($name:ident($(($input:expr, $output:expr)),+ $(,)?)) => {
        paste::item! {
            #[test]
            fn [< test_type_mapping_ $name >] () {
                $(
                    let dml = format!(r#"
                        datasource db {{
                            provider = "sqlserver"
                            url = "sqlserver://"
                        }}

                        generator js {{
                            provider = "prisma-client-js"
                            previewFeatures = ["nativeTypes"]
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

                    let result = MsSqlType::try_from(instance).unwrap();

                    assert_eq!($output, result);
                )+
            }
        }
    };
}

test_type!(tinyint(("Int @db.TinyInt", MsSqlType::tinyint())));
test_type!(smallint(("Int @db.SmallInt", MsSqlType::smallint())));
test_type!(int(("Int @db.Int", MsSqlType::int())));
test_type!(money(("Decimal @db.Money", MsSqlType::money())));
test_type!(smallmoney(("Decimal @db.SmallMoney", MsSqlType::smallmoney())));
test_type!(real(("Float @db.Real", MsSqlType::real())));
test_type!(date(("DateTime @db.Date", MsSqlType::date())));
test_type!(time(("DateTime @db.Time", MsSqlType::time())));
test_type!(datetime(("DateTime @db.DateTime", MsSqlType::datetime())));
test_type!(datetime2(("DateTime @db.DateTime2", MsSqlType::datetime2())));
test_type!(text(("String @db.Text", MsSqlType::text())));
test_type!(ntext(("String @db.NText", MsSqlType::ntext())));
test_type!(image(("Bytes @db.Image", MsSqlType::image())));
test_type!(xml(("String @db.Xml", MsSqlType::xml())));

test_type!(datetimeoffset((
    "DateTime @db.DateTimeOffset",
    MsSqlType::datetimeoffset()
)));

test_type!(smalldatetime((
    "DateTime @db.SmallDateTime",
    MsSqlType::smalldatetime()
)));

test_type!(binary(
    ("Bytes @db.Binary", MsSqlType::binary(None)),
    ("Bytes @db.Binary(4000)", MsSqlType::binary(Some(4000)))
));

test_type!(varbinary(
    ("Bytes @db.VarBinary", MsSqlType::varbinary(Option::<u64>::None)),
    ("Bytes @db.VarBinary(4000)", MsSqlType::varbinary(Some(4000u64))),
    (
        "Bytes @db.VarBinary(Max)",
        MsSqlType::varbinary(Some(TypeParameter::Max))
    ),
));

test_type!(char(
    ("String @db.Char", MsSqlType::r#char(None)),
    ("String @db.Char(4000)", MsSqlType::r#char(Some(4000)))
));

test_type!(nchar(
    ("String @db.NChar", MsSqlType::nchar(None)),
    ("String @db.NChar(2000)", MsSqlType::nchar(Some(2000)))
));

test_type!(varchar(
    ("String @db.VarChar", MsSqlType::varchar(Option::<u64>::None)),
    ("String @db.VarChar(4000)", MsSqlType::varchar(Some(4000u64))),
    ("String @db.VarChar(Max)", MsSqlType::varchar(Some(TypeParameter::Max))),
));

test_type!(nvarchar(
    ("String @db.NVarChar", MsSqlType::nvarchar(Option::<u64>::None)),
    ("String @db.NVarChar(2000)", MsSqlType::nvarchar(Some(2000u64))),
    (
        "String @db.NVarChar(Max)",
        MsSqlType::nvarchar(Some(TypeParameter::Max))
    ),
));

test_type!(boolean(
    ("Boolean @db.Bit", MsSqlType::bit()),
    ("Int @db.Bit", MsSqlType::bit()),
));

test_type!(decimal(
    ("Decimal @db.Decimal", MsSqlType::decimal(None)),
    ("Decimal @db.Decimal(32,16)", MsSqlType::decimal(Some((32, 16)))),
));

test_type!(number(
    ("Decimal @db.Numeric", MsSqlType::numeric(None)),
    ("Decimal @db.Numeric(32,16)", MsSqlType::numeric(Some((32, 16)))),
));

test_type!(float(
    ("Float @db.Float", MsSqlType::float(None)),
    ("Float @db.Float(24)", MsSqlType::float(Some(24))),
    ("Float @db.Float(53)", MsSqlType::float(Some(53))),
));
