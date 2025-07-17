use indoc::indoc;
use sql_introspection_tests::{TestResult, test_api::*};
use test_macros::test_connector;

const TYPES: &[(&str, &str)] = &[
    //fieldname, db datatype
    ("int", "Int"),
    ("smallint", "SmallInt"),
    ("tinyint", "TinyInt"),
    ("bigint", "BigInt"),
    ("decimal", "Decimal(5,3)"),
    ("decimal_2", "Decimal"),
    ("numeric", "Numeric(4,1)"),
    ("numeric_2", "Numeric"),
    ("money", "Money"),
    ("smallmoney", "SmallMoney"),
    ("float", "Real"),
    ("double", "Float(53)"),
    ("bit", "Bit"),
    ("chars", "Char(10)"),
    ("nchars", "NChar(10)"),
    ("varchars", "VarChar(500)"),
    ("varchars_2", "VarChar(Max)"),
    ("nvarchars", "NVarChar(500)"),
    ("nvarchars_2", "NVarChar(Max)"),
    ("binary", "Binary(230)"),
    ("varbinary", "VarBinary(150)"),
    ("varbinary_2", "VarBinary(Max)"),
    ("date", "Date"),
    ("time", "Time"),
    ("datetime", "DateTime"),
    ("datetime2", "DateTime2"),
    ("xml", "Xml"),
    ("image", "Image"),
    ("text", "Text"),
    ("ntext", "NText"),
];

#[test_connector(tags(Mssql))]
async fn native_type_columns_feature_on(api: &mut TestApi) -> TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, db_type)| format!("[{name}] {db_type} NOT NULL"))
        .collect();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.inject_custom("id INT IDENTITY, CONSTRAINT [Blog_pkey] PRIMARY KEY ([id])");

                for column in &columns {
                    t.inject_custom(column);
                }
            });
        })
        .await?;

    let types = indoc! {r#"
        model Blog {
          id          Int      @id @default(autoincrement())
          int         Int
          smallint    Int      @db.SmallInt
          tinyint     Int      @db.TinyInt
          bigint      BigInt
          decimal     Decimal  @db.Decimal(5, 3)
          decimal_2   Decimal  @db.Decimal(18, 0)
          numeric     Decimal  @db.Decimal(4, 1)
          numeric_2   Decimal  @db.Decimal(18, 0)
          money       Float    @db.Money
          smallmoney  Float    @db.SmallMoney
          float       Float    @db.Real
          double      Float
          bit         Boolean
          chars       String   @db.Char(10)
          nchars      String   @db.NChar(10)
          varchars    String   @db.VarChar(500)
          varchars_2  String   @db.VarChar(Max)
          nvarchars   String   @db.NVarChar(500)
          nvarchars_2 String   @db.NVarChar(Max)
          binary      Bytes    @db.Binary(230)
          varbinary   Bytes    @db.VarBinary(150)
          varbinary_2 Bytes
          date        DateTime @db.Date
          time        DateTime @db.Time
          datetime    DateTime @db.DateTime
          datetime2   DateTime
          xml         String   @db.Xml
          image       Bytes    @db.Image
          text        String   @db.Text
          ntext       String   @db.NText
        }
    "#};

    let result = api.introspect().await?;

    println!("EXPECTATION: \n {types:#}");
    println!("RESULT: \n {result:#}");

    api.assert_eq_datamodels(types, &result);

    Ok(())
}
