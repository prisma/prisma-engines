use bigdecimal::BigDecimal;
use chrono::Utc;
use migration_engine_tests::sql::*;
use once_cell::sync::Lazy;
use quaint::{
    prelude::{Insert, Queryable},
    Value,
};
use std::{collections::HashMap, str::FromStr};

// define everything as safe first
// define typemaps
// run tests and see which ones explode

static ALL: &[&'static str] = &[
    "SmallInt",
    "Integer",
    "BigInt",
    "Decimal(32,16)",
    "Numeric(32,16)",
    "Real",
    "DoublePrecision",
    // "SmallSerial", todo you cannot use this in alter column -.- have to do smallint autoincrement????
    // "BigSerial",  todo same here
    "VarChar(53)",
    "Char(53)",
    "Text",
    "ByteA",
    "Timestamp(3)",
    "Timestamptz(3)",
    "Date",
    "Time(3)",
    "Timetz(3)",
    "Boolean",
    "Bit(10)",
    "VarBit(10)",
    "UUID",
    "Xml",
    "JSON",
    "JSONB",
];

static SAFE_CASTS: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        (
            "SmallInt",
            Value::integer(u8::MAX),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Numeric(32,16)",
                "Real",
                "DoublePrecision",
                // "SmallSerial", todo you cannot use this in alter column -.- have to do smallint autoincrement????
                // "BigSerial",  todo same here
                "VarChar(53)",
                "Char(53)",
                "Text",
                // "ByteA",
                // "Timestamp(3)",
                // "Timestamptz(3)",
                // "Date",
                // "Time(3)",
                "Timetz(3)",
                "Boolean",
                "Bit(10)",
                "VarBit(10)",
                "UUID",
                "Xml",
                "JSON",
                "JSONB",
            ],
        ),
        ("Integer", Value::integer(i32::MAX), ALL),
        ("BigInt", Value::integer(i64::MAX), ALL),
        ("Decimal(10,2)", Value::numeric(BigDecimal::from_str("1").unwrap()), ALL),
        ("Numeric(11,4)", Value::numeric(BigDecimal::from_str("1").unwrap()), ALL),
        ("Real", Value::float(5.3), ALL),
        ("DoublePrecision", Value::float(7.5), ALL),
        ("SmallSerial", Value::integer(10), ALL),
        ("Serial", Value::integer(15), ALL),
        ("BigSerial", Value::integer(i64::MAX), ALL),
        ("VarChar(5)", Value::text("true"), ALL),
        ("Char(5)", Value::text("true"), ALL),
        ("Text", Value::text("true"), ALL),
        ("ByteA", Value::bytes(vec![1]), ALL),
        ("Timestamp(3)", Value::datetime(Utc::now()), ALL),
        ("Timestamptz(3)", Value::datetime(Utc::now()), ALL),
        ("Date", Value::date(Utc::today().naive_utc()), ALL),
        ("Time(3)", Value::time(Utc::now().naive_utc().time()), ALL),
        ("Timetz(3)", Value::time(Utc::now().naive_utc().time()), ALL),
        ("Boolean", Value::boolean(true), ALL),
        ("Bit(10)", Value::bytes(vec![1]), ALL),
        ("VarBit(5)", Value::bytes(vec![1]), ALL),
        ("UUID", Value::text("75bf0037-a8b8-4512-beea-5a186f8abf1e"), ALL),
        ("Xml", Value::boolean(true), ALL),
        ("JSON", Value::boolean(true), ALL),
        ("JSONB", Value::boolean(true), ALL),
    ]
});

static TYPE_MAPS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut maps = HashMap::new();

    maps.insert("SmallInt", "Int");
    maps.insert("Integer", "Int");
    maps.insert("BigInt", "BigInt");
    maps.insert("Decimal", "Decimal");
    maps.insert("Numeric", "Decimal");
    maps.insert("Real", "Float");
    maps.insert("DoublePrecision", "Float");
    maps.insert("SmallSerial", "Int");
    maps.insert("BigSerial", "Int");
    maps.insert("VarChar", "String");
    maps.insert("Char", "String");
    maps.insert("Text", "String");
    maps.insert("ByteA", "Bytes");
    maps.insert("Timestamp", "DateTime");
    maps.insert("Timestamptz", "DateTime");
    maps.insert("Date", "DateTime");
    maps.insert("Time", "DateTime");
    maps.insert("Timetz", "DateTime");
    maps.insert("Boolean", "Boolean");
    maps.insert("Bit", "Bytes");
    maps.insert("VarBit", "Bytes");
    maps.insert("UUID", "String");
    maps.insert("Xml", "String");
    maps.insert("JSON", "Json");
    maps.insert("JSONB", "Json");

    maps
});

fn with_default_params(r#type: &str) -> &str {
    println!("{}", r#type);
    match r#type {
        "SmallInt" => "int2",
        "Integer" => "int4",
        "BigInt" => "int8",
        "Decimal(32,16)" => "numeric",
        "Numeric(32,16)" => "numeric",
        "Real" => "float4",
        "DoublePrecision" => "float8",
        "VarChar(53)" => "varchar",
        "Char(53)" => "bpchar",
        "VarBinary" => "VarBinary(1)",
        "NVarChar" => "NVarChar(1)",
        "Char" => "Char(1)",
        "NChar" => "NChar(1)",
        _ => r#type,
    }
}

fn prisma_type(native_type: &str) -> &str {
    let kind = native_type.split("(").next().unwrap();
    TYPE_MAPS.get(kind).unwrap()
}

#[test_each_connector(tags("postgres"), features("native_types"))]
async fn safe_casts_with_existing_data_should_work(api: &TestApi) -> TestResult {
    for (from, seed, casts) in SAFE_CASTS.iter() {
        for to in *casts {
            println!("From `{}` to `{}` with seed `{:?}`", from, to, seed);

            let dm1 = api.native_types_datamodel(format!(
                r#"
                model A {{
                    id Int @id @default(autoincrement()) @test_db.Integer
                    x  {prisma_type} @test_db.{native_type}
                }}
                "#,
                prisma_type = prisma_type(from),
                native_type = from,
            ));

            api.schema_push(&dm1).send().await?.assert_green()?;

            let insert = Insert::single_into((api.schema_name(), "A")).value("x", seed.clone());
            api.database().insert(insert.into()).await?;

            api.assert_schema().await?.assert_table("A", |table| {
                table.assert_columns_count(2)?.assert_column("x", |c| {
                    c.assert_is_required()?
                        .assert_native_type(&with_default_params(from).to_lowercase())
                })
            })?;

            let dm2 = api.native_types_datamodel(format!(
                r#"
                model A {{
                    id Int @id @default(autoincrement()) @test_db.Integer
                    x  {prisma_type} @test_db.{native_type}
                }} 
                "#,
                prisma_type = prisma_type(to),
                native_type = to,
            ));

            api.schema_push(&dm2).send().await?.assert_green()?;

            api.assert_schema().await?.assert_table("A", |table| {
                table.assert_columns_count(2)?.assert_column("x", |c| {
                    c.assert_is_required()?
                        .assert_native_type(&with_default_params(to).to_lowercase())
                })
            })?;

            api.database()
                .raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()))
                .await?;
        }
    }

    Ok(())
}
