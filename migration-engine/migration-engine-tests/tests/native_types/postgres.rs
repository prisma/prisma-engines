use bigdecimal::BigDecimal;
use chrono::Utc;
use migration_engine_tests::sql::*;
use once_cell::sync::Lazy;
use quaint::{
    prelude::{Insert, Queryable},
    Value,
};
use std::{collections::HashMap, str::FromStr};
use migration_engine_tests::AssertionResult;

// refactor testcase to run all mappings for one type at once
// do all safe number casts
// setup number tests for risky
// setup number tests for non-castable
// start thinking about list->scalar , scalar -> list

static ALL: &[&'static str] = &[
    "SmallInt",
    "Integer",
    "BigInt",
    "Decimal(32,16)",
    "Numeric(32,16)",
    "Real",
    "DoublePrecision",
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
    "Uuid",
    "Xml",
    "Json",
    "JsonB",
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
                // "Decimal(32,16)", //todo risky
                // "Numeric(32,16)", //todo risky
                "Real",
                "DoublePrecision",
                // "VarChar(53)", //todo risky
                // "Char(53)",    //todo risky
                "Text",
                // "ByteA",
                // "Timestamp(3)",
                // "Timestamptz(3)",
                // "Date",
                // "Time(3)",
                // "Timetz(3)",
                // "Boolean",
                // "Bit(10)",
                // "VarBit(10)",
                // "Uuid",
                // "Xml",
                // "Json",
                // "JsonB",
            ],
        ),
        (
            "Integer",
            Value::integer(i32::MAX),
            &[
                // "SmallInt", // todo risky
                "Integer",
                "BigInt",
                // "Decimal(32,16)",//todo risky
                // "Numeric(32,16)",//todo risky
                "Real",
                "DoublePrecision",
                // "VarChar(53)", //todo risky
                // "Char(53)",    //todo risky
                "Text",
                // "ByteA",
                // "Timestamp(3)",
                // "Timestamptz(3)",
                // "Date",
                // "Time(3)",
                // "Timetz(3)",
                // "Boolean",
                // "Bit(10)",
                // "VarBit(10)",
                // "Uuid",
                // "Xml",
                // "Json",
                // "JsonB",
            ],
        ),
        (
            "BigInt",
            Value::integer(i64::MAX),
            &[
                // "SmallInt", // todo risky
                // "Integer", // todo risky
                "BigInt",
                // "Decimal(32,16)",//todo risky
                // "Numeric(32,16)", //todo risky
                "Real",
                "DoublePrecision",
                // "VarChar(53)", //todo risky
                // "Char(53)",    //todo risky
                "Text",
                // "ByteA",
                // "Timestamp(3)",
                // "Timestamptz(3)",
                // "Date",
                // "Time(3)",
                // "Timetz(3)",
                // "Boolean",
                // "Bit(10)",
                // "VarBit(10)",
                // "Uuid",
                // "Xml",
                // "Json",
                // "JsonB",
            ],
        ),
        (
            "Decimal(10,2)",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &[
                // "SmallInt", // todo risky
                // "Integer", // todo risky
                // "BigInt",// todo risky
                "Decimal(32,16)",
                "Numeric(32,16)",
                // "Real", // todo risky
                // "DoublePrecision",// todo risky
                "VarChar(53)",
                "Char(53)",
                "Text",
                // "ByteA",
                // "Timestamp(3)",
                // "Timestamptz(3)",
                // "Date",
                // "Time(3)",
                // "Timetz(3)",
                // "Boolean",
                // "Bit(10)",
                // "VarBit(10)",
                // "Uuid",
                // "Xml",
                // "Json",
                // "JsonB",
            ],
        ),
        (
            "Numeric(5,0)",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Numeric(32,16)",
                // "Real", // todo not risky due to params
                // "DoublePrecision",// todo not risky due to params
                "VarChar(53)",
                "Char(53)",
                "Text",
                // "ByteA",
                // "Timestamp(3)",
                // "Timestamptz(3)",
                // "Date",
                // "Time(3)",
                // "Timetz(3)",
                // "Boolean",
                // "Bit(10)",
                // "VarBit(10)",
                // "Uuid",
                // "Xml",
                // "Json",
                // "JsonB",
            ],
        ),
        (
            "Real",
            Value::float(5.3),
            &[
                // "SmallInt",  //todo risky
                // "Integer",  //todo risky
                // "BigInt", //todo risky
                // "Decimal(32,16)", //todo not risky
                // "Numeric(32,16)",//todo not risky
                "Real",
                "DoublePrecision",
                "VarChar(53)", //todo
                "Char(53)",    // todo
                "Text",
                "ByteA",
                // "Timestamp(3)",
                // "Timestamptz(3)",
                // "Date",
                // "Time(3)",
                // "Timetz(3)",
                // "Boolean",
                // "Bit(10)",
                // "VarBit(10)",
                // "Uuid",
                // "Xml",
                // "Json",
                // "JsonB",
            ],
        ),
        (
            "DoublePrecision",
            Value::float(7.5),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Numeric(32,16)",
                "Real",
                "DoublePrecision",
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
                "Uuid",
                "Xml",
                "Json",
                "JsonB",
            ],
        ),
        //todo later
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
        ("Uuid", Value::text("75bf0037-a8b8-4512-beea-5a186f8abf1e"), ALL),
        ("Xml", Value::boolean(true), ALL),
        ("Json", Value::boolean(true), ALL),
        ("JsonB", Value::boolean(true), ALL),
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
    maps.insert("Bit", "String");
    maps.insert("VarBit", "String");
    maps.insert("Uuid", "String");
    maps.insert("Xml", "String");
    maps.insert("Json", "Json");
    maps.insert("JsonB", "Json");

    maps
});

fn with_default_params(r#type: &str) -> &str {
    println!("{}", r#type);
    match r#type {
        "SmallInt" => "int2",
        "Integer" => "int4",
        "BigInt" => "int8",
        "Decimal(32,16)" => "numeric",
        "Decimal(10,2)" => "numeric",
        "Numeric(32,16)" => "numeric",
        "Numeric(5,0)" => "numeric",
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
        let mut previous_columns = "".to_string();
        let mut next_columns = "".to_string();
        let mut insert = Insert::single_into((api.schema_name(), "A"));
        let mut previous_assertions = vec![];
        // let mut next_assertions = vec![];

        for (idx, to) in casts.iter().enumerate() {
            println!("From `{}` to `{}` with seed `{:?}`", from, to, seed);

            let column_name = format!("column_{}", idx);

            previous_columns.push_str(&format!(
                "{column_name}  {prisma_type}? @test_db.{native_type} \n",
                prisma_type = prisma_type(from),
                native_type = from,
                column_name = column_name
            ));

            next_columns.push_str(&format!(
                "{column_name}  {prisma_type}? @test_db.{native_type}\n",
                prisma_type = prisma_type(to),
                native_type = to,
                column_name = column_name
            ));

            insert = insert.value(column_name, seed.clone());

            previous_assertions.push((&column_name, with_default_params(from).to_lowercase()));
            //
            // next_assertions.push(|table: TableAssertion| {
            //     table.assert_column(&column_name, |c| {
            //             .assert_native_type(&with_default_params(to).to_lowercase())
            //     })
            // });
        }

        let dm1 = api.native_types_datamodel(format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @test_db.Integer
                    {columns}
                }}
                "#,
            columns = previous_columns
        ));

        api.schema_push(&dm1).send().await?.assert_green()?;

        //inserts
        api.database().insert(insert.into()).await?;

        //first assertions
        // api.assert_schema().await?.assert_table("A", |table| {
        //
        //
        //     table.assert_column(column_name, |c| c.assert_native_type(native_type))
        //     // previous_assertions.iter().map(|(column_name, native_type)|{
        //     //
        //     // table.assert_column(column_name, |c| c.assert_native_type(native_type)
        //     // })
        // })?;

        let dm2 = api.native_types_datamodel(format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @test_db.Integer
                    {columns}
                }}
                "#,
            columns = next_columns
        ));

        api.schema_push(&dm2).send().await?.assert_green()?;

        //second assertions

        api.assert_schema().await?.assert_table("A", |table| {
            previous_assertions.iter().fold(_, |acc, |)

            table.assert_column("x", |c| {
                    c.assert_native_type(&with_default_params(to).to_lowercase())
            }).and_then(table.assert_column("x", |c| {
                c.assert_native_type(&with_default_params(to).to_lowercase())
            }))
        })?;

        api.database()
            .raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()))
            .await?;
    }

    Ok(())
}
