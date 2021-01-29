use indoc::formatdoc;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_datamodels, test_api::*};
use test_macros::test_each_connector;
use test_setup::connectors::Tags;

const TYPES: &[(&str, &str)] = &[
    //fieldname, db datatype
    ("int", "int(11)"),
    ("unsignedint", "int(12) unsigned"),
    ("smallint", "SmallInt"),
    ("unsignedsmallint", "SmallInt unsigned"),
    ("tinyint", "TinyInt"),
    ("unsignedtinyint", "TinyInt unsigned"),
    ("tinyint_bool", "TinyInt(1)"),
    ("mediumint", "MediumInt"),
    ("unsignedmediumint", "MediumInt unsigned"),
    ("bigint", "BigInt"),
    ("bigint_autoincrement", "BigInt Auto_Increment Primary Key"),
    ("unsignedbigint", "BigInt unsigned"),
    ("decimal", "Decimal(5, 3)"),
    ("decimal_2", "Decimal"),
    ("numeric", "Decimal(4,1)"),
    ("float", "Float"),
    ("double", "Double"),
    ("bits", "Bit(64)"),
    ("bit_bool", "Bit(1)"),
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
    ("dateTimeWithPrecision", "DateTime(3)"),
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
            int                            Int
            unsignedint                    Int      @mysql.UnsignedInt
            smallint                       Int      @mysql.SmallInt
            unsignedsmallint               Int      @mysql.UnsignedSmallInt
            tinyint                        Int      @mysql.TinyInt
            unsignedtinyint                Int      @mysql.UnsignedTinyInt
            tinyint_bool                   Boolean
            mediumint                      Int      @mysql.MediumInt
            unsignedmediumint              Int      @mysql.UnsignedMediumInt
            bigint                         BigInt
            bigint_autoincrement           BigInt   @id  @default(autoincrement())
            unsignedbigint                 BigInt   @mysql.UnsignedBigInt
            decimal                        Decimal  @mysql.Decimal(5, 3)
            decimal_2                      Decimal  @mysql.Decimal(10, 0)
            numeric                        Decimal  @mysql.Decimal(4, 1)
            float                          Float    @mysql.Float
            double                         Float
            bits                           Bytes    @mysql.Bit(64)
            bit_bool                       Boolean  @mysql.Bit(1)
            chars                          String   @mysql.Char(10)
            varchars                       String   @mysql.VarChar(500)
            binary                         Bytes    @mysql.Binary(230)
            varbinary                      Bytes    @mysql.VarBinary(150)
            tinyBlob                       Bytes    @mysql.TinyBlob
            blob                           Bytes    @mysql.Blob
            mediumBlob                     Bytes    @mysql.MediumBlob
            longBlob                       Bytes
            tinytext                       String   @mysql.TinyText
            text                           String   @mysql.Text
            mediumText                     String   @mysql.MediumText
            longText                       String   @mysql.LongText
            date                           DateTime @mysql.Date
            timeWithPrecision              DateTime @mysql.Time(3)
            timeWithPrecision_no_precision DateTime @mysql.DateTime(0)
            dateTimeWithPrecision          DateTime
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
            int                            Int
            unsignedint                    Int
            smallint                       Int
            unsignedsmallint               Int
            tinyint                        Int
            unsignedtinyint                Int
            tinyint_bool                   Boolean
            mediumint                      Int
            unsignedmediumint              Int
            bigint                         Int
            bigint_autoincrement           Int       @id  @default(autoincrement())
            unsignedbigint                 Int
            decimal                        Float
            decimal_2                      Float
            numeric                        Float
            float                          Float
            double                         Float
            bits                           Int
            bit_bool                       Int
            chars                          String
            varchars                       String
            // This type is currently not supported by the Prisma Client.
            // binary                      Unsupported("binary(230)")
            // This type is currently not supported by the Prisma Client.
            // varbinary                   Unsupported("varbinary(150)")
            // This type is currently not supported by the Prisma Client.
            // tinyBlob                    Unsupported("tinyblob")
            // This type is currently not supported by the Prisma Client.
            // blob                        Unsupported("blob")
            // This type is currently not supported by the Prisma Client.
            // mediumBlob                  Unsupported("mediumblob")
            // This type is currently not supported by the Prisma Client.
            // longBlob                    Unsupported("longblob")
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
