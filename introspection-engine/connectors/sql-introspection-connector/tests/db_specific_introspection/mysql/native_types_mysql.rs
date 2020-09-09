use crate::*;
use test_harness::*;

const TYPES: &'static [(&str, &str)] = &[
    //fieldname, db datatype, non-native result, native result
    ("int", "int(11)"),
    ("smallint", "SmallInt"),
    ("tinyint", "TinyInt"),
    ("mediumint", "MediumInt"),
    ("bigint", "BigInt"),
    ("decimal", "Decimal(5, 3)"),
    ("numeric", "Decimal(4,1)"),
    ("float", "Float"),
    ("double", "Double"),
    ("bits", "Bit(10)"),
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
];

#[test_each_connector(tags("mysql"))]
async fn introspecting_native_type_columns_feature_on(api: &TestApi) -> TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, db_type)| format!("`{}` {} Not Null", name, db_type))
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

    let dm = r#"
       model Blog {
         id                             Int      @id
         int                            Int
         smallint                       Int
         tinyint                        Int
         mediumint                      Int
         bigint                         Int
         // This type is currently not supported.
         // decimal                     decimal
         // This type is currently not supported.
         // numeric                     decimal
         float                          Float
         double                         Float
         // This type is currently not supported.
         // bits                        bytes
         chars                          String
         varchars                       String
         // This type is currently not supported.
         // binary                      bytes
         // This type is currently not supported.
         // varbinary                   bytes
         // This type is currently not supported.
         // tinyBlob                    bytes
         // This type is currently not supported.
         // blob                        bytes
         // This type is currently not supported.
         // mediumBlob                  bytes
         // This type is currently not supported.
         // longBlob                    bytes
         tinytext                       String
         text                           String
         mediumText                     String
         longText                       String
         date                           DateTime
         timeWithPrecision              DateTime
         timeWithPrecision_no_precision DateTime
         dateTimeWithPrecision          DateTime
         timestampWithPrecision         DateTime @default(now())
         year                           Int
    }
    "#
    .to_owned();

    let result = dbg!(api.re_introspect(&dm).await);
    assert_eq!(dm.replace(" ", ""), result.replace(" ", ""));

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_native_type_columns_feature_off(api: &TestApi) -> TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, data_type)| format!("`{}` {} Not Null", name, data_type))
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

    let dm = r#"datasource mysql {
   provider = "mysql"
   url      = "mysql://localhost/test"
 }
 
 model Blog {
       id                             Int      @id
   int                            Int
   smallint                       Int
   tinyint                        Int
   mediumint                      Int
   bigint                         Int
   // This type is currently not supported.
   // decimal                     decimal
   // This type is currently not supported.
   // numeric                     decimal
   float                          Float
   double                         Float
   // This type is currently not supported.
   // bits                        bytes
   chars                          String
   varchars                       String
   // This type is currently not supported.
   // binary                      bytes
   // This type is currently not supported.
   // varbinary                   bytes
   // This type is currently not supported.
   // tinyBlob                    bytes
   // This type is currently not supported.
   // blob                        bytes
   // This type is currently not supported.
   // mediumBlob                  bytes
   // This type is currently not supported.
   // longBlob                    bytes
   tinytext                       String
   text                           String
   mediumText                     String
   longText                       String
   date                           DateTime
   timeWithPrecision              DateTime
   timeWithPrecision_no_precision DateTime
   dateTimeWithPrecision          DateTime
   timestampWithPrecision         DateTime
   year                           Int
 }
    "#
    .to_owned();

    let result = dbg!(api.re_introspect(&dm).await);
    assert_eq!(dm.replace(" ", ""), result.replace(" ", ""));

    Ok(())
}
