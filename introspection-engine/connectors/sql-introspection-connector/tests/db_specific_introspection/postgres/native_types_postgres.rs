use crate::*;
use test_harness::*;

// ("smallint", "Int", "SmallInt", "int2"),
// ("int", "Int", "Integer", "int4"),
// ("bigint", "Int", "BigInt", "int8"),
// ("decimal", "Decimal", "Decimal(4, 2)", "numeric"),
// ("numeric", "Decimal", "Numeric(4, 2)", "numeric"),
// ("real", "Float", "Real", "float4"),
// ("doublePrecision", "Float", "DoublePrecision", "float8"),
// ("smallSerial", "Int", "SmallSerial", "int2"),
// ("serial", "Int", "Serial", "int4"),
// ("bigSerial", "Int", "BigSerial", "int8"),
// ("varChar", "String", "VarChar(200)", "varchar"),
// ("char", "String", "Char(200)", "bpchar"),
// ("text", "String", "Text", "text"),
// ("bytea", "Bytes", "ByteA", "bytea"),
// ("ts", "DateTime", "Timestamp(0)", "timestamp"),
// ("tstz", "DateTime", "TimestampWithTimeZone(0)", "timestamptz"),
// ("date", "DateTime", "Date", "date"),
// ("time", "DateTime", "Time(2)", "time"),
// ("timetz", "DateTime", "TimeWithTimeZone(2)", "timetz"),
// ("interval", "Duration", "Interval(2)", "interval"),
// ("bool", "Boolean", "Boolean", "bool"),
// ("bit", "String", "Bit(1)", "bit"),
// ("varbit", "String", "VarBit(1)", "varbit"),
// ("uuid", "String", "Uuid", "uuid"),
// ("xml", "XML", "Xml", "xml"),
// ("json", "Json", "Json", "json"),
// ("jsonb", "Json", "JsonB", "jsonb"),

const TYPES: &'static [(&str, &str)] = &[
    //fieldname, db datatype
    ("smallint", "SmallInt"),
    ("int", "Integer"),
    ("bigint", "BigInt"),
    ("decimal", "Decimal(4, 2)"),
    ("numeric", "Numeric(4, 2)"),
    ("real", "Real"),
    ("doublePrecision", "Double Precision"),
    ("smallSerial", "SmallSerial"),
    ("serial", "Serial"),
    ("bigSerial", "BigSerial"),
    ("varChar", "VarChar(200)"),
    ("char", "Char(200)"),
    ("text", "Text"),
    ("bytea", "ByteA"),
    ("ts", "Timestamp(0)"),
    ("tstz", "Timestamptz(0)"),
    ("date", "Date"),
    ("time", "Time(2)"),
    ("timetz", "Timetz(2)"),
    ("interval", "Interval(2)"),
    ("bool", "Boolean"),
    ("bit", "Bit(1)"),
    ("varbit", "VarBit(1)"),
    ("uuid", "Uuid"),
    ("xml", "Xml"),
    ("json", "Json"),
    ("jsonb", "JsonB"),
];

#[test_each_connector(tags("postgres"))]
async fn introspecting_native_type_columns_feature_on(api: &TestApi) -> TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, db_type)| format!("\"{}\" {} Not Null", name, db_type))
        .collect();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.inject_custom("id Integer Primary Key");
                for column in &columns {
                    t.inject_custom(column);
                }
            });
        })
        .await;

    let mut dm = r#"datasource postgres {		        
    provider        = "postgres"		       
    url             = "postgres://localhost/test"		         
    previewFeatures = ["nativeTypes"]		          
 }
"#
    .to_owned();

    let types = r#"
      model Blog {
  id                             Int      @id @mysql.Int
  int                            Int      @mysql.Int
  smallint                       Int      @mysql.SmallInt
  tinyint                        Int      @mysql.TinyInt
  mediumint                      Int      @mysql.MediumInt
  bigint                         Int      @mysql.BigInt
  decimal                        Decimal  @mysql.Decimal(5, 3)
  numeric                        Decimal  @mysql.Decimal(4, 1)
  float                          Float    @mysql.Float
  double                         Float    @mysql.Double
  bits                           Bytes    @mysql.Bit(10)
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
  timeWithPrecision_no_precision DateTime @mysql.Datetime
  dateTimeWithPrecision          DateTime @mysql.Datetime(3)
  timestampWithPrecision         DateTime @default(now()) @mysql.Timestamp(3)
  year                           Int      @mysql.Year
}
"#;

    //Fixme parsing can't handle native types yet???
    let result = dbg!(api.re_introspect(&dm).await);

    dm.push_str(types);

    println!("EXPECTATION: \n {}", dm);
    println!("RESULT: \n {}", result);
    assert_eq!(result.replace(" ", "").contains(&types.replace(" ", "")), true);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_native_type_columns_feature_off(api: &TestApi) -> TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, data_type)| format!("\"{}\" {} Not Null", name, data_type))
        .collect();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.inject_custom("id Integer Primary Key");
                for column in &columns {
                    t.inject_custom(column);
                }
            });
        })
        .await;

    let dm = r#"model Blog {
  id                             Int            @id
  int                            Int
  smallint                       Int
  tinyint                        Int
  mediumint                      Int
  bigint                         Int
  decimal                        Float
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
  timestampWithPrecision         DateTime       @default(now())
  year                           Int
}
"#
    .to_owned();

    let result = dbg!(api.re_introspect(&dm).await);

    println!("EXPECTATION: \n {}", dm);
    println!("RESULT: \n {}", result);
    assert_eq!(dm.replace(" ", ""), result.replace(" ", ""));

    Ok(())
}
