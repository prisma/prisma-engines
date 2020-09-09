use crate::*;
use std::fmt::Write;
use test_harness::*;

#[test_each_connector(tags("mysql"))]
async fn introspecting_native_type_columns_works(api: &TestApi) -> TestResult {
    let types = &[
        // ("int", "Int", "Int", if api.is_mysql_8() { "int" } else { "int(11)" }),
        ("int", "Int", "Int", "int(11)"),
        (
            "smallint",
            "Int",
            "SmallInt",
            // if api.is_mysql_8() { "smallint" } else { "smallint(6)" },
            "smallint(6)",
        ),
        (
            "tinyint",
            "Int",
            "TinyInt",
            // if api.is_mysql_8() { "tinyint" } else { "tinyint(4)" },
            "tinyint(4)",
        ),
        (
            "mediumint",
            "Int",
            "MediumInt",
            // if api.is_mysql_8() { "mediumint" } else { "mediumint(9)" },
            "mediumint(9)",
        ),
        (
            "bigint",
            "Int",
            "BigInt",
            // if api.is_mysql_8() { "bigint" } else { "bigint(20)" },
            "bigint(20)",
        ),
        ("decimal", "Decimal", "Decimal(5, 3)", "decimal(5,3)"),
        ("numeric", "Decimal", "Decimal(4,1)", "numeric(4,1)"),
        ("float", "Float", "Float", "float"),
        ("double", "Float", "Double", "double"),
        ("bits", "Bytes", "Bit(10)", "bit(10)"),
        ("chars", "String", "Char(10)", "char(10)"),
        ("varchars", "String", "VarChar(500)", "varchar(500)"),
        ("binary", "Bytes", "Binary(230)", "binary(230)"),
        ("varbinary", "Bytes", "VarBinary(150)", "varbinary(150)"),
        ("tinyBlob", "Bytes", "TinyBlob", "tinyblob"),
        ("blob", "Bytes", "Blob", "blob"),
        ("mediumBlob", "Bytes", "MediumBlob", "mediumblob"),
        ("longBlob", "Bytes", "LongBlob", "longblob"),
        ("tinytext", "String", "TinyText", "tinytext"),
        ("text", "String", "Text", "text"),
        ("mediumText", "String", "MediumText", "mediumtext"),
        ("longText", "String", "LongText", "longtext"),
        ("date", "DateTime", "Date", "date"),
        ("timeWithPrecision", "DateTime", "Time(3)", "time(3)"),
        ("timeWithPrecision_no_precision", "DateTime", "Time", "time"),
        ("dateTimeWithPrecision", "DateTime", "Datetime(3)", "datetime(3)"),
        ("timestampWithPrecision", "DateTime", "Timestamp(3)", "timestamp(3)"),
        // ("year", "Int", "Year", if api.is_mysql_8() { "year" } else { "year(4)" }),
        ("year", "Int", "Year", "year(4)"),
    ];

    let columns: Vec<String> = types
        .iter()
        .map(|(name, _, tpe, _)| format!("`{}` {} Not Null", name, tpe))
        .collect();

    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            move |migration| {
                migration.create_table("Blog", move |t| {
                    t.inject_custom("id Integer Primary Key");
                    for column in &columns {
                        t.inject_custom(column);
                    }
                });
            },
            api.db_name(),
        )
        .await;

    let mut dm = r#"
        datasource mysql {
            provider = "mysql"
            url = "mysql://localhost/test"
            previewFeatures = ["nativeTypes"]
        }
        
        model Blog {
            id Int @id @mysql.Int
    "#
    .to_owned();

    for (field_name, prisma_type, native_type, _) in types {
        writeln!(&mut dm, "    {} {} @mysql.{}", field_name, prisma_type, native_type)?;
    }

    dm.push_str(
        "}
    ",
    );

    let result = dbg!(api.re_introspect(&dm).await);
    custom_assert_with_config(&result, &dm);

    Ok(())
}
