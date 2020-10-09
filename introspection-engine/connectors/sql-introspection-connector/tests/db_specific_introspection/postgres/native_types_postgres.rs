use crate::*;
use test_harness::*;

const TYPES: &[(&str, &str)] = &[
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
  id              Int      @id @postgres.Integer
  smallint        Int      @postgres.SmallInt
  int             Int      @postgres.Integer
  bigint          Int      @postgres.BigInt
  decimal         Decimal  @postgres.Numeric(4, 2)
  numeric         Decimal  @postgres.Numeric(4, 2)
  real            Float    @postgres.Real
  doublePrecision Float    @postgres.DoublePrecision
  smallSerial     Int      @default(autoincrement()) @postgres.SmallInt
  serial          Int      @default(autoincrement()) @postgres.Integer
  bigSerial       Int      @default(autoincrement()) @postgres.BigInt
  varChar         String   @postgres.VarChar(200)
  char            String   @postgres.Char(200)
  text            String   @postgres.Text
  bytea           Bytes    @postgres.ByteA
  ts              DateTime @postgres.Timestamp(0)
  tstz            DateTime @postgres.TimestampWithTimeZone(0)
  date            DateTime @postgres.Date
  time            DateTime @postgres.Time(2)
  timetz          DateTime @postgres.TimeWithTimeZone(2)
  interval        Duration @postgres.Interval(2)
  bool            Boolean  @postgres.Boolean
  bit             String   @postgres.Bit(1)
  varbit          String   @postgres.VarBit(1)
  uuid            String   @postgres.Uuid
  // This type is currently not supported.
  // xml          xml
  json            Json     @postgres.Json
  jsonb           Json     @postgres.JsonB
}
"#;

    let result = dbg!(api.re_introspect(&dm).await);

    //Fixme parsing can't handle native types yet???
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

    let mut dm = r#"datasource postgres {		        
    provider        = "postgres"		       
    url             = "postgres://localhost/test"		         
 }"#
    .to_owned();

    let types = r#"
    model Blog {
  id              Int      @id
  smallint        Int
  int             Int
  bigint          Int
  decimal         Float
  numeric         Float
  real            Float
  doublePrecision Float
  smallSerial     Int      @default(autoincrement())
  serial          Int      @default(autoincrement())
  bigSerial       Int      @default(autoincrement())
  varChar         String
  char            String
  text            String
  // This type is currently not supported.
  // bytea        bytea
  ts              DateTime
  tstz            DateTime
  date            DateTime
  time            DateTime
  timetz          DateTime
  interval        String
  bool            Boolean
  bit             String
  varbit          String
  uuid            String
  // This type is currently not supported.
  // xml          xml
  json            Json
  jsonb           Json
}
"#
    .to_owned();

    let result = dbg!(api.re_introspect(&dm).await);

    dm.push_str(&types);

    println!("EXPECTATION: \n {}", dm);
    println!("RESULT: \n {}", result);
    assert_eq!(result.replace(" ", "").contains(&types.replace(" ", "")), true);

    Ok(())
}
