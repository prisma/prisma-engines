use crate::*;
use test_harness::*;

const TYPES: &'static [(&str, &str)] = &[
    //fieldname, db datatype
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

#[test_each_connector(tags("mysql_5_6", "mariadb"))]
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

    let mut dm = r#"datasource mysql {		        
    provider        = "mysql"		       
    url             = "mysql://localhost/test"		         
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
  timeWithPrecision_no_precision DateTime @mysql.Datetime(0)
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

#[test_each_connector(tags("mysql_5_6", "mariadb"))]
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
