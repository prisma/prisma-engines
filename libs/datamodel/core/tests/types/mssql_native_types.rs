use crate::{
    common::*,
    types::helper::{
        test_native_types_compatibility, test_native_types_with_field_attribute_support,
        test_native_types_without_attributes,
    },
    with_header, Provider,
};
use indoc::indoc;
use native_types::{MsSqlType, MsSqlTypeParameter::*};

const BLOB_TYPES: &[&str] = &["VarBinary(Max)", "Image"];
const TEXT_TYPES: &[&str] = &["Text", "NText", "VarChar(Max)", "NVarChar(Max)", "Xml"];

#[test]
fn text_and_blob_data_types_should_fail_on_index() {
    fn error_msg(type_name: &str) -> String {
        format!(
            "You cannot define an index on fields with native type `{}` of SQL Server.",
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
        format!("Native type `{}` cannot be unique in SQL Server.", type_name)
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
            "Native type `{}` of SQL Server cannot be used on a field that is `@id` or `@@id`.",
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
    let dml = indoc! {r#"
        model Blog {
          id  Int     @id
          dec Decimal @test.Decimal(2,4)
        }
    "#};

    let dml = with_header(dml, Provider::SqlServer, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe scale must not be larger than the precision for the Decimal(2,4) native type in SQL Server.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id  Int     @id
        [1;94m13 | [0m  dec Decimal @[1;91mtest.Decimal(2,4)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn should_fail_on_argument_out_of_range_for_char_type() {
    let error_msg =
        "Argument M is out of range for native type `Char(8001)` of SQL Server: Length can range from 1 to 8000.";

    test_native_types_without_attributes("Char(8001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_nchar_type() {
    let error_msg =
        "Argument M is out of range for native type `NChar(4001)` of SQL Server: Length can range from 1 to 4000.";

    test_native_types_without_attributes("NChar(4001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varchar_type() {
    let error_msg = "Argument M is out of range for native type `VarChar(8001)` of SQL Server: Length can range from 1 to 8000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("VarChar(8001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_nvarchar_type() {
    let error_msg = "Argument M is out of range for native type `NVarChar(4001)` of SQL Server: Length can range from 1 to 4000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("NVarChar(4001)", "String", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_varbinary_type() {
    let error_msg = "Argument M is out of range for native type `VarBinary(8001)` of SQL Server: Length can range from 1 to 8000. For larger sizes, use the `Max` variant.";

    test_native_types_without_attributes("VarBinary(8001)", "Bytes", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_argument_out_of_range_for_binary_type() {
    let error_msg =
        "Argument M is out of range for native type `Binary(8001)` of SQL Server: Length can range from 1 to 8000.";

    test_native_types_without_attributes("Binary(8001)", "Bytes", error_msg, MSSQL_SOURCE);
}

#[test]
fn should_fail_on_incompatible_scalar_type_with_tiny_int() {
    let dml = indoc! {r#"
        model Blog {
            id     Int      @id
            bigInt DateTime @test.Bit
        }
    "#};

    let dml = with_header(dml, Provider::SqlServer, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type Bit is not compatible with declared field type DateTime, expected field type Boolean or Int.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int      @id
        [1;94m13 | [0m    bigInt DateTime @[1;91mtest.Bit[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn should_fail_on_bad_type_params() {
    let dml = indoc! {r#"
        model Blog {
          id     Int    @id
          s      String @test.NVarChar(Ma)
        }
    "#};

    let dml = with_header(dml, Provider::SqlServer, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mInvalid argument for type NVarChar: Ma. Allowed values: a number or `Max`.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id     Int    @id
        [1;94m13 | [0m  s      String @[1;91mtest.NVarChar(Ma)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn should_fail_on_too_many_type_params() {
    let dml = indoc! {r#"
        model Blog {
          id     Int    @id
          s      String @test.NVarChar(1, 2)
        }
    "#};

    let dml = with_header(dml, Provider::SqlServer, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNative type NVarChar takes 1 optional arguments, but received 2.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id     Int    @id
        [1;94m13 | [0m  s      String @[1;91mtest.NVarChar(1, 2)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

macro_rules! test_type {
    ($name:ident($(($input:expr, $output:expr)),+ $(,)?)) => {
            #[test]
            fn $name () {
                $(
                    let dml = format!(r#"
                        model Blog {{
                            id Int    @id
                            x  {}
                        }}
                    "#, $input);

                    let dml = with_header(&dml, Provider::SqlServer, &[]);

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

    test_type!(tinyint(("Int @test.TinyInt", MsSqlType::TinyInt)));
    test_type!(smallint(("Int @test.SmallInt", MsSqlType::SmallInt)));
    test_type!(int(("Int @test.Int", MsSqlType::Int)));
    test_type!(money(("Float @test.Money", MsSqlType::Money)));
    test_type!(smallmoney(("Float @test.SmallMoney", MsSqlType::SmallMoney)));
    test_type!(real(("Float @test.Real", MsSqlType::Real)));
    test_type!(date(("DateTime @test.Date", MsSqlType::Date)));
    test_type!(time(("DateTime @test.Time", MsSqlType::Time)));
    test_type!(datetime(("DateTime @test.DateTime", MsSqlType::DateTime)));
    test_type!(datetime2(("DateTime @test.DateTime2", MsSqlType::DateTime2)));
    test_type!(text(("String @test.Text", MsSqlType::Text)));
    test_type!(ntext(("String @test.NText", MsSqlType::NText)));
    test_type!(image(("Bytes @test.Image", MsSqlType::Image)));
    test_type!(xml(("String @test.Xml", MsSqlType::Xml)));

    test_type!(datetimeoffset((
        "DateTime @test.DateTimeOffset",
        MsSqlType::DateTimeOffset
    )));

    test_type!(smalldatetime((
        "DateTime @test.SmallDateTime",
        MsSqlType::SmallDateTime
    )));

    test_type!(binary(
        ("Bytes @test.Binary", MsSqlType::Binary(None)),
        ("Bytes @test.Binary(4000)", MsSqlType::Binary(Some(4000)))
    ));

    test_type!(varbinary(
        ("Bytes @test.VarBinary", MsSqlType::VarBinary(None)),
        ("Bytes @test.VarBinary(4000)", MsSqlType::VarBinary(Some(Number(4000)))),
        ("Bytes @test.VarBinary(Max)", MsSqlType::VarBinary(Some(Max))),
    ));

    test_type!(char(
        ("String @test.Char", MsSqlType::Char(None)),
        ("String @test.Char(4000)", MsSqlType::Char(Some(4000)))
    ));

    test_type!(nchar(
        ("String @test.NChar", MsSqlType::NChar(None)),
        ("String @test.NChar(4000)", MsSqlType::NChar(Some(4000)))
    ));

    test_type!(varchar(
        ("String @test.VarChar", MsSqlType::VarChar(None)),
        ("String @test.VarChar(8000)", MsSqlType::VarChar(Some(Number(8000)))),
        ("String @test.VarChar(Max)", MsSqlType::VarChar(Some(Max))),
    ));

    test_type!(nvarchar(
        ("String @test.NVarChar", MsSqlType::NVarChar(None)),
        ("String @test.NVarChar(4000)", MsSqlType::NVarChar(Some(Number(4000)))),
        ("String @test.NVarChar(Max)", MsSqlType::NVarChar(Some(Max))),
    ));

    test_type!(boolean(
        ("Boolean @test.Bit", MsSqlType::Bit),
        ("Int @test.Bit", MsSqlType::Bit),
    ));

    test_type!(decimal(
        ("Decimal @test.Decimal", MsSqlType::Decimal(None)),
        ("Decimal @test.Decimal(32,16)", MsSqlType::Decimal(Some((32, 16)))),
    ));

    test_type!(number(
        ("Decimal @test.Decimal", MsSqlType::Decimal(None)),
        ("Decimal @test.Decimal(32,16)", MsSqlType::Decimal(Some((32, 16)))),
    ));

    test_type!(float(
        ("Float @test.Float", MsSqlType::Float(None)),
        ("Float @test.Float(24)", MsSqlType::Float(Some(24))),
        ("Float @test.Float(53)", MsSqlType::Float(Some(53))),
    ));
}
