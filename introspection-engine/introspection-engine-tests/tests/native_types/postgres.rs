use indoc::indoc;
use introspection_engine_tests::test_api::*;
use test_macros::test_each_connector_mssql as test_each_connector;

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
    ("tstz", "Timestamptz(2)"),
    ("date", "Date"),
    ("time", "Time(2)"),
    ("time_2", "Time"),
    ("timetz", "Timetz(2)"),
    ("bool", "Boolean"),
    ("bit", "Bit(1)"),
    ("varbit", "VarBit(1)"),
    ("uuid", "Uuid"),
    ("xml", "Xml"),
    ("json", "Json"),
    ("jsonb", "JsonB"),
];

#[test_each_connector(tags("postgres"))]
async fn native_type_columns_feature_on(api: &TestApi) -> crate::TestResult {
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
        .await?;

    let mut dm = indoc! {r#"
        datasource postgres {
            provider        = "postgres"
            url             = "postgres://localhost/test"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["nativeTypes"]
        }
    "#}
    .to_string();

    let types = indoc! {r#"
        model Blog {
            id              Int      @id @postgres.Integer
            smallint        Int      @postgres.SmallInt
            int             Int      @postgres.Integer
            bigint          BigInt   @postgres.BigInt
            decimal         Decimal  @postgres.Numeric(4, 2)
            numeric         Decimal  @postgres.Numeric(4, 2)
            real            Float    @postgres.Real
            doublePrecision Float    @postgres.DoublePrecision
            smallSerial     Int      @default(autoincrement()) @postgres.SmallInt
            serial          Int      @default(autoincrement()) @postgres.Integer
            bigSerial       BigInt   @default(autoincrement()) @postgres.BigInt
            varChar         String   @postgres.VarChar(200)
            char            String   @postgres.Char(200)
            text            String   @postgres.Text
            bytea           Bytes    @postgres.ByteA
            ts              DateTime @postgres.Timestamp(0)
            tstz            DateTime @postgres.Timestamptz(2)
            date            DateTime @postgres.Date
            time            DateTime @postgres.Time(2)
            time_2          DateTime @postgres.Time(6)
            timetz          DateTime @postgres.Timetz(2)
            bool            Boolean  @postgres.Boolean
            bit             String   @postgres.Bit(1)
            varbit          String   @postgres.VarBit(1)
            uuid            String   @postgres.Uuid
            xml             String   @postgres.Xml
            json            Json     @postgres.Json
            jsonb           Json     @postgres.JsonB
          }
    "#};

    let result = api.re_introspect(&dm).await?;

    dm.push_str(types);

    println!("EXPECTATION: \n {:#}", dm);
    println!("RESULT: \n {:#}", result);

    assert!(result.replace(" ", "").contains(&types.replace(" ", "")));

    Ok(())
}

#[test_each_connector(tags("postgres"))]

async fn native_type_columns_feature_off(api: &TestApi) -> crate::TestResult {
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
        .await?;

    let mut dm = indoc! {r#"
        datasource postgres {
            provider        = "postgres"
            url             = "postgres://localhost/test"
        }
    "#}
    .to_string();

    let types = indoc! {r#"
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
            time_2          DateTime
            timetz          DateTime
            bool            Boolean
            bit             String
            varbit          String
            uuid            String
            xml             String
            json            Json
            jsonb           Json
        }
    "#};

    let result = api.re_introspect(&dm).await?;

    dm.push_str(types);

    println!("EXPECTATION: \n {:#}", dm);
    println!("RESULT: \n {:#}", result);

    assert!(result.replace(" ", "").contains(&types.replace(" ", "")));

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn native_type_array_columns_feature_on(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.inject_custom("id Integer Primary Key");
                t.inject_custom("decimal_array Decimal(42,0)[] ");
                t.inject_custom("decimal_array_2 Decimal[] ");
                t.inject_custom("numeric_array Numeric(4, 2)[] ");
                t.inject_custom("numeric_array_2 Numeric[] ");
                t.inject_custom("varchar_array Varchar(42)[] ");
                t.inject_custom("varchar_array_2 Varchar[] ");
                t.inject_custom("char_array Char(200)[] ");
                t.inject_custom("char_array_2 Char[] ");
                t.inject_custom("bit_array Bit(20)[] ");
                t.inject_custom("bit_array_2 Bit[] ");
                t.inject_custom("varbit_array Varbit(2)[] ");
                t.inject_custom("varbit_array_2 Varbit[] ");
                t.inject_custom("timestamp_array Timestamp(4)[] ");
                t.inject_custom("time_array Time(4)[] ");
            });
        })
        .await?;

    let dm = indoc! {r#"
         generator client {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
         }

         datasource postgres {
            provider        = "postgresql"
            url             = "postgres://localhost/test"
         }

         model Blog {
          id                Int        @id @postgres.Integer
          decimal_array     Decimal[]  @postgres.Numeric(42, 0)
          decimal_array_2   Decimal[]  @postgres.Numeric
          numeric_array     Decimal[]  @postgres.Numeric(4, 2)
          numeric_array_2   Decimal[]  @postgres.Numeric
          varchar_array     String[]   @postgres.VarChar(42)
          varchar_array_2   String[]   @postgres.VarChar
          char_array        String[]   @postgres.Char(200)
          char_array_2      String[]   @postgres.Char(1)
          bit_array         String[]   @postgres.Bit(20)
          bit_array_2       String[]   @postgres.Bit(1)
          varbit_array      String[]   @postgres.VarBit(2)
          varbit_array_2    String[]   @postgres.VarBit
          timestamp_array   DateTime[] @postgres.Timestamp(4)
          time_array        DateTime[] @postgres.Time(4)
        }
    "#}
    .to_string();

    let result = api.re_introspect(&dm).await?;

    println!("EXPECTATION: \n {:#}", dm);
    println!("RESULT: \n {:#}", result);

    assert!(result.replace(" ", "").contains(&dm.replace(" ", "")));

    Ok(())
}
