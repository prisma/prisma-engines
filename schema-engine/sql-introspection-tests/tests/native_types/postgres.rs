use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

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
    ("money", "Money"),
    ("oid", "Oid"),
    ("inet", "Inet"),
];

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn native_type_columns_feature_on(api: &mut TestApi) -> TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, db_type)| format!("\"{name}\" {db_type} Not Null"))
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

    let types = indoc! {r#"
        model Blog {
            id              Int      @id
            smallint        Int      @db.SmallInt
            int             Int
            bigint          BigInt
            decimal         Decimal  @db.Decimal(4, 2)
            numeric         Decimal  @db.Decimal(4, 2)
            real            Float    @db.Real
            doublePrecision Float
            smallSerial     Int      @default(autoincrement()) @db.SmallInt
            serial          Int      @default(autoincrement())
            bigSerial       BigInt   @default(autoincrement())
            varChar         String   @db.VarChar(200)
            char            String   @db.Char(200)
            text            String
            bytea           Bytes
            ts              DateTime @db.Timestamp(0)
            tstz            DateTime @db.Timestamptz(2)
            date            DateTime @db.Date
            time            DateTime @db.Time(2)
            time_2          DateTime @db.Time(6)
            timetz          DateTime @db.Timetz(2)
            bool            Boolean
            bit             String   @db.Bit(1)
            varbit          String   @db.VarBit(1)
            uuid            String   @db.Uuid
            xml             String   @db.Xml
            json            Json     @db.Json
            jsonb           Json
            money           Decimal  @db.Money
            oid             Int      @db.Oid
            inet            String   @db.Inet
          }
    "#};

    let result = api.introspect().await?;

    println!("EXPECTATION: \n {types:#}");
    println!("RESULT: \n {result:#}");

    api.assert_eq_datamodels(types, &result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn native_type_array_columns_feature_on(api: &mut TestApi) -> TestResult {
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

    let types = indoc! {r#"
         model Blog {
          id                Int        @id
          decimal_array     Decimal[]  @db.Decimal(42, 0)
          decimal_array_2   Decimal[]  @db.Decimal
          numeric_array     Decimal[]  @db.Decimal(4, 2)
          numeric_array_2   Decimal[]  @db.Decimal
          varchar_array     String[]   @db.VarChar(42)
          varchar_array_2   String[]   @db.VarChar
          char_array        String[]   @db.Char(200)
          char_array_2      String[]   @db.Char(1)
          bit_array         String[]   @db.Bit(20)
          bit_array_2       String[]   @db.Bit(1)
          varbit_array      String[]   @db.VarBit(2)
          varbit_array_2    String[]   @db.VarBit
          timestamp_array   DateTime[] @db.Timestamp(4)
          time_array        DateTime[] @db.Time(4)
        }
    "#}
    .to_string();

    let result = api.introspect().await?;

    println!("EXPECTATION: \n {types:#}");
    println!("RESULT: \n {result:#}");

    api.assert_eq_datamodels(&types, &result);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn cdb_char_is_a_char(api: &mut TestApi) -> TestResult {
    // https://github.com/prisma/prisma/issues/12281

    api.normalise_int_type().await?;

    api.barrel()
        .execute(move |migration| {
            migration.create_table("Blog", move |t| {
                t.inject_custom("id Integer Primary Key");
                t.inject_custom(r#"ch "char" DEFAULT 'Y'::"char" NOT NULL"#);
            });
        })
        .await?;

    let result = api.introspect_dml().await?;

    let expected = expect![[r#"
        model Blog {
          id Int    @id
          ch String @default("Y") @db.CatalogSingleChar
        }
    "#]];

    expected.assert_eq(&result);

    Ok(())
}
