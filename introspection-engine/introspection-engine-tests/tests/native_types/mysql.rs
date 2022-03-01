use indoc::formatdoc;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

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

#[test_connector(tags(Mariadb, Mysql8))]
async fn native_type_columns_feature_on(api: &TestApi) -> TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, db_type)| format!("`{}` {} Not Null", name, db_type))
        .collect();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                for column in &columns {
                    t.inject_custom(column);
                }
            });
        })
        .await?;

    let (json, default) = match api {
        _ if api.tags().contains(Tags::Mysql8) => ("Json", ""),
        _ if api.tags().contains(Tags::Mariadb) => ("String   @db.LongText", "@default(now())"),
        _ => unreachable!(),
    };

    let types = formatdoc! {r#"
        model Blog {{
            int                            Int
            unsignedint                    Int      @db.UnsignedInt
            smallint                       Int      @db.SmallInt
            unsignedsmallint               Int      @db.UnsignedSmallInt
            tinyint                        Int      @db.TinyInt
            unsignedtinyint                Int      @db.UnsignedTinyInt
            tinyint_bool                   Boolean
            mediumint                      Int      @db.MediumInt
            unsignedmediumint              Int      @db.UnsignedMediumInt
            bigint                         BigInt
            bigint_autoincrement           BigInt   @id  @default(autoincrement())
            unsignedbigint                 BigInt   @db.UnsignedBigInt
            decimal                        Decimal  @db.Decimal(5, 3)
            decimal_2                      Decimal  @db.Decimal(10, 0)
            numeric                        Decimal  @db.Decimal(4, 1)
            float                          Float    @db.Float
            double                         Float
            bits                           Bytes    @db.Bit(64)
            bit_bool                       Boolean  @db.Bit(1)
            chars                          String   @db.Char(10)
            varchars                       String   @db.VarChar(500)
            binary                         Bytes    @db.Binary(230)
            varbinary                      Bytes    @db.VarBinary(150)
            tinyBlob                       Bytes    @db.TinyBlob
            blob                           Bytes    @db.Blob
            mediumBlob                     Bytes    @db.MediumBlob
            longBlob                       Bytes
            tinytext                       String   @db.TinyText
            text                           String   @db.Text
            mediumText                     String   @db.MediumText
            longText                       String   @db.LongText
            date                           DateTime @db.Date
            timeWithPrecision              DateTime @db.Time(3)
            timeWithPrecision_no_precision DateTime @db.DateTime(0)
            dateTimeWithPrecision          DateTime
            timestampWithPrecision         DateTime {default} @db.Timestamp(3)
            year                           Int      @db.Year
            json                           {json}
        }}
    "#,
    default = default,
    json = json
    };

    let result = api.introspect().await?;

    println!("EXPECTATION: \n {:#}", types);
    println!("RESULT: \n {:#}", result);

    api.assert_eq_datamodels(&types, &result);

    Ok(())
}
