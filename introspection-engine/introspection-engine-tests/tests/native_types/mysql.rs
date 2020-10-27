use indoc::formatdoc;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, test_api::*};
use test_macros::test_each_connector_mssql as test_each_connector;
use test_setup::connectors::Tags;

const TYPES: &[(&str, &str)] = &[
    //fieldname, db datatype
    ("int", "int(11)"),
    ("smallint", "SmallInt"),
    ("tinyint", "TinyInt"),
    ("tinyint_bool", "TinyInt(1)"),
    ("mediumint", "MediumInt"),
    ("bigint", "BigInt"),
    ("decimal", "Decimal(5, 3)"),
    ("decimal_2", "Decimal"),
    ("numeric", "Decimal(4,1)"),
    ("float", "Float"),
    ("double", "Double"),
    ("bits", "Bit(64)"),
    ("chars", "Char(10)"),
    ("varchars", "VarChar(500)"),
    ("binary", "Binary(230)"),
    ("varbinary", "VarBinary(150)"),
    ("tinyBlob", "TinyBlob"),
    ("blob", "Blob"),
    ("mediumBlob", "MediumBlob"),
    ("longBlob", "LongBlob"),
    ("tinytext", "TinyText"),
    ("text", "Text"),
    ("mediumText", "MediumText"),
    ("longText", "LongText"),
    ("date", "Date"),
    ("timeWithPrecision", "Time(3)"),
    ("timeWithPrecision_no_precision", "DateTime"),
    ("dateTimeWithPrecision", "Datetime(3)"),
    ("timestampWithPrecision", "Timestamp(3)"),
    ("year", "Year"),
    ("json", "Json"),
];

#[test_each_connector(tags("mariadb", "mysql_8"))]
async fn native_type_columns_feature_on(api: &TestApi) -> crate::TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, db_type)| format!("`{}` {} Not Null", name, db_type))
        .collect();

    api.barrel()
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
        .await?;

    let (json, default) = match api {
        _ if api.tags.contains(Tags::Mysql8) => ("Json     @mysql.JSON", ""),
        _ if api.tags.contains(Tags::Mariadb) => ("String   @mysql.LongText", "@default(now())"),
        _ => unreachable!(),
    };

    let mut dm = String::from(indoc! {r#"
        datasource mysql {
            provider        = "mysql"
            url             = "mysql://localhost/test"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["nativeTypes"]
        }
    "#});

    let types = formatdoc! {r#"
        model Blog {{
            id                             Int      @id @mysql.Int
            int                            Int      @mysql.Int
            smallint                       Int      @mysql.SmallInt
            tinyint                        Int      @mysql.TinyInt
            tinyint_bool                   Boolean  @mysql.TinyInt
            mediumint                      Int      @mysql.MediumInt
            bigint                         Int      @mysql.BigInt
            decimal                        Decimal  @mysql.Decimal(5, 3)
            decimal_2                      Decimal  @mysql.Decimal(10, 0)
            numeric                        Decimal  @mysql.Decimal(4, 1)
            float                          Float    @mysql.Float
            double                         Float    @mysql.Double
            bits                           Bytes    @mysql.Bit(64)
            chars                          String   @mysql.Char(10)
            varchars                       String   @mysql.VarChar(500)
            binary                         Bytes    @mysql.Binary(230)
            varbinary                      Bytes    @mysql.VarBinary(150)
            tinyBlob                       Bytes    @mysql.TinyBlob
            blob                           Bytes    @mysql.Blob
            mediumBlob                     Bytes    @mysql.MediumBlob
            longBlob                       Bytes    @mysql.LongBlob
            tinytext                       String   @mysql.TinyText
            text                           String   @mysql.Text
            mediumText                     String   @mysql.MediumText
            longText                       String   @mysql.LongText
            date                           DateTime @mysql.Date
            timeWithPrecision              DateTime @mysql.Time(3)
            timeWithPrecision_no_precision DateTime @mysql.Datetime(0)
            dateTimeWithPrecision          DateTime @mysql.Datetime(3)
            timestampWithPrecision         DateTime {default} @mysql.Timestamp(3)
            year                           Int      @mysql.Year
            json                           {json}
        }}
    "#,
    default = default,
    json = json
    };

    let result = api.re_introspect(&dm).await?;

    dm.push_str(&types);

    println!("EXPECTATION: \n {:#}", types);
    println!("RESULT: \n {:#}", result);

    assert!(result.replace(" ", "").contains(&types.replace(" ", "")));

    Ok(())
}

#[test_each_connector(tags("mariadb", "mysql_8"))]
async fn native_type_columns_feature_off(api: &TestApi) -> crate::TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, data_type)| format!("`{}` {} Not Null", name, data_type))
        .collect();

    api.barrel()
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
        .await?;

    let (json, default) = match api {
        _ if api.tags.contains(Tags::Mysql8) => ("Json", ""),
        _ if api.tags.contains(Tags::Mariadb) => ("String", "@default(now())"),
        _ => unreachable!(),
    };

    let dm = formatdoc! {r#"
        datasource mysql {{
            provider        = "mysql"
            url             = "mysql://localhost/test"
        }}


        model Blog {{
            id                             Int            @id
            int                            Int
            smallint                       Int
            tinyint                        Int   
            tinyint_bool                   Boolean
            mediumint                      Int
            bigint                         Int
            decimal                        Float
            decimal_2                      Float
            numeric                        Float
            float                          Float
            double                         Float
            bits                           Int
            chars                          String
            varchars                       String
            // This type is currently not supported.
            // binary                      binary(230)
            // This type is currently not supported.
            // varbinary                   varbinary(150)
            // This type is currently not supported.
            // tinyBlob                    tinyblob
            // This type is currently not supported.
            // blob                        blob
            // This type is currently not supported.
            // mediumBlob                  mediumblob
            // This type is currently not supported.
            // longBlob                    longblob
            tinytext                       String
            text                           String
            mediumText                     String
            longText                       String
            date                           DateTime
            timeWithPrecision              DateTime
            timeWithPrecision_no_precision DateTime
            dateTimeWithPrecision          DateTime
            timestampWithPrecision         DateTime       {default}
            year                           Int
            json                           {json}
        }}
    "#,
    default = default,
    json = json
    };

    assert_eq_datamodels!(&dm, &api.re_introspect(&dm).await?);

    Ok(())
}
