use bigdecimal::BigDecimal;
use migration_engine_tests::sql::*;
use once_cell::sync::Lazy;
use quaint::{
    prelude::{Insert, Queryable},
    Value,
};
use std::{collections::HashMap, str::FromStr};

// start thinking about list->scalar , scalar -> list

// static ALL: &[&'static str] =
//     &[
//     "SmallInt",
//     "Integer",
//     "BigInt",
//     "Decimal(32,16)",
//     "Numeric(32,16)",
//     "Real",
//     "DoublePrecision",
//     "VarChar(53)",
//     "Char(53)",
//     "Text",
//     "ByteA",
//     "Timestamp(3)",
//     "Timestamptz(3)",
//     "Date",
//     "Time(3)",
//     "Timetz(3)",
//     "Boolean",
//     "Bit(10)",
//     "VarBit(10)",
//     "Uuid",
//     "Xml",
//     "Json",
//     "JsonB",
// ];

static SAFE_CASTS: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        (
            "SmallInt",
            Value::integer(u8::MAX),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Real",
                "DoublePrecision",
                "VarChar(53)",
                "Char(53)",
                "Text",
            ],
        ),
        (
            "Integer",
            Value::integer(i32::MAX),
            &[
                "Integer",
                "BigInt",
                "Real",
                "DoublePrecision",
                "VarChar(53)",
                "Char(53)",
                "Text",
            ],
        ),
        (
            "BigInt",
            Value::integer(i64::MAX),
            &["BigInt", "Real", "DoublePrecision", "VarChar(53)", "Char(53)", "Text"],
        ),
        (
            "Decimal(10,2)",
            Value::numeric(BigDecimal::from_str("12345678.90").unwrap()),
            &["Decimal(32,16)", "VarChar(53)", "Char(53)", "Text"],
        ),
        (
            "Numeric(3,0)",
            Value::numeric(BigDecimal::from_str("123").unwrap()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Numeric(32,16)",
                "VarChar(53)",
                "Char(53)",
                "Text",
            ],
        ),
        (
            "Real",
            Value::float(f32::MIN),
            &["DoublePrecision", "VarChar(53)", "Char(53)", "Text"],
        ),
        (
            "DoublePrecision",
            Value::Double(Some(f64::MIN)),
            &["DoublePrecision", "Text"],
        ),
        ("VarChar(5)", Value::text("fiver"), &["VarChar(53)", "Char(53)", "Text"]),
        ("Char(5)", Value::text("truer"), &["VarChar(53)", "Char(53)", "Text"]),
        ("Text", Value::text("true"), &["VarChar", "Text"]),
        ("ByteA", Value::bytes(vec![1]), &["Text", "VarChar"]),
        //todo later
        // ("Timestamp(3)", Value::datetime(Utc::now()), ALL),
        // ("Timestamptz(3)", Value::datetime(Utc::now()), ALL),
        // ("Date", Value::date(Utc::today().naive_utc()), ALL),
        // ("Time(3)", Value::time(Utc::now().naive_utc().time()), ALL),
        // ("Timetz(3)", Value::time(Utc::now().naive_utc().time()), ALL),
        // ("Boolean", Value::boolean(true), ALL),
        // ("Bit(10)", Value::bytes(vec![1]), ALL),
        // ("VarBit(5)", Value::bytes(vec![1]), ALL),
        // ("Uuid", Value::text("75bf0037-a8b8-4512-beea-5a186f8abf1e"), ALL),
        // ("Xml", Value::boolean(true), ALL),
        // ("Json", Value::boolean(true), ALL),
        // ("JsonB", Value::boolean(true), ALL),
    ]
});

static RISKY_CASTS: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        (
            "SmallInt",
            Value::integer(i8::MIN),
            &["Numeric(2,1)", "VarChar(3)", "Char"],
        ),
        (
            "Integer",
            Value::integer(i32::MIN),
            &["Decimal(4,3)", "Numeric(2,1)", "VarChar(4)", "Char"],
        ),
        (
            "BigInt",
            Value::integer(i64::MIN),
            &["Decimal(10,9)", "Numeric(2,1)", "VarChar(17)", "Char"],
        ),
        (
            "Decimal(10,2)",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &["SmallInt", "Integer", "BigInt", "Real", "DoublePrecision"],
        ),
        (
            "Numeric(5,0)",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &[
                "Real",            // todo not risky due to params
                "DoublePrecision", // todo not risky due to params
            ],
        ),
        (
            "Real",
            Value::float(f32::MAX),
            &["SmallInt", "Integer", "BigInt", "Decimal(32,16)", "Numeric(32,16)"],
        ),
        (
            "DoublePrecision",
            Value::double(f64::MAX),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Numeric(32,16)",
                "Real",
                "VarChar(53)",
                "Char(53)",
            ],
        ),
        //todo later
        // ("VarChar(5)", Value::text("true"), ALL),
        // ("Char(5)", Value::text("true"), ALL),
        // ("Text", Value::text("true"), ALL),
        ("ByteA", Value::bytes(vec![1]), &["VarChar(53)", "Char(53)"]),
        // ("Timestamp(3)", Value::datetime(Utc::now()), ALL),
        // ("Timestamptz(3)", Value::datetime(Utc::now()), ALL),
        // ("Date", Value::date(Utc::today().naive_utc()), ALL),
        // ("Time(3)", Value::time(Utc::now().naive_utc().time()), ALL),
        // ("Timetz(3)", Value::time(Utc::now().naive_utc().time()), ALL),
        // ("Boolean", Value::boolean(true), ALL),
        // ("Bit(10)", Value::bytes(vec![1]), ALL),
        // ("VarBit(5)", Value::bytes(vec![1]), ALL),
        // ("Uuid", Value::text("75bf0037-a8b8-4512-beea-5a186f8abf1e"), ALL),
        // ("Xml", Value::boolean(true), ALL),
        // ("Json", Value::boolean(true), ALL),
        // ("JsonB", Value::boolean(true), ALL),
    ]
});

static NOT_CASTABLE: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        (
            "SmallInt",
            Value::integer(u8::MAX),
            &[
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
        (
            "Integer",
            Value::integer(i32::MAX),
            &[
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
        (
            "BigInt",
            Value::integer(i64::MAX),
            &[
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
        (
            "Decimal(10,2)",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &[
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
        (
            "Numeric(5,0)",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &[
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
        (
            "Real",
            Value::float(5.3),
            &[
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
        (
            "DoublePrecision",
            Value::float(7.5),
            &[
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
        // ("VarChar(5)", Value::text("true"), ALL),
        // ("Char(5)", Value::text("true"), ALL),
        // ("Text", Value::text("true"), ALL),
        // ("ByteA", Value::bytes(vec![1]), ALL),
        // ("Timestamp(3)", Value::datetime(Utc::now()), ALL),
        // ("Timestamptz(3)", Value::datetime(Utc::now()), ALL),
        // ("Date", Value::date(Utc::today().naive_utc()), ALL),
        // ("Time(3)", Value::time(Utc::now().naive_utc().time()), ALL),
        // ("Timetz(3)", Value::time(Utc::now().naive_utc().time()), ALL),
        // ("Boolean", Value::boolean(true), ALL),
        // ("Bit(10)", Value::bytes(vec![1]), ALL),
        // ("VarBit(5)", Value::bytes(vec![1]), ALL),
        // ("Uuid", Value::text("75bf0037-a8b8-4512-beea-5a186f8abf1e"), ALL),
        // ("Xml", Value::boolean(true), ALL),
        // ("Json", Value::boolean(true), ALL),
        // ("JsonB", Value::boolean(true), ALL),
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
    match r#type {
        "SmallInt" => "int2",
        "Integer" => "int4",
        "BigInt" => "int8",
        "VarChar(5)" => "varchar",
        "Decimal(32,16)" => "numeric",
        "Decimal(10,2)" => "numeric",
        "Numeric(32,16)" => "numeric",
        "Numeric(3,0)" => "numeric",
        "Numeric(2,1)" => "DECIMAL(2, 1)",
        "Decimal(4,3)" => "DECIMAL(4, 3)",
        "Decimal(10,9)" => "DECIMAL(10, 9)",
        "Real" => "float4",
        "DoublePrecision" => "float8",
        "VarChar(53)" => "varchar",
        "VarChar(3)" => "VARCHAR(3)",
        "VarChar(4)" => "VARCHAR(4)",
        "VarChar(17)" => "VARCHAR(17)",
        "Char(53)" => "bpchar",
        "Char(5)" => "bpchar",
        "Char(1)" => "CHAR",
        "VarBinary" => "VarBinary(1)",
        "Char" => "CHAR",
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
        let mut next_assertions = vec![];

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

            insert = insert.value(column_name.clone(), seed.clone());

            previous_assertions.push((column_name.clone(), with_default_params(from).to_lowercase()).clone());
            next_assertions.push((column_name.clone(), with_default_params(to).to_lowercase()).clone());
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
        api.assert_schema().await?.assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_column_count(previous_assertions.len() + 1),
                |acc, (column_name, expected)| {
                    acc.and_then(|table| table.assert_column(column_name, |c| c.assert_native_type(expected)))
                },
            )
        })?;

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
            next_assertions.iter().fold(
                table.assert_column_count(next_assertions.len() + 1),
                |acc, (name, tpe)| acc.and_then(|table| table.assert_column(name, |c| c.assert_native_type(tpe))),
            )
        })?;

        api.database()
            .raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()))
            .await?;
    }

    Ok(())
}

#[test_each_connector(tags("postgres"), features("native_types"))]
async fn risky_casts_with_existing_data_should_warn(api: &TestApi) -> TestResult {
    //todo here we seed the columns with data
    // but since every single column switch is risky and we do not force
    // we don't execute the migration. This probably should be split into:
    // - risky and fails with force
    // - risky but ultimately succeeds
    for (from, seed, casts) in RISKY_CASTS.iter() {
        let mut previous_columns = "".to_string();
        let mut next_columns = "".to_string();
        let mut insert = Insert::single_into((api.schema_name(), "A"));
        let mut previous_assertions = vec![];
        let mut warnings = vec![];

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

            insert = insert.value(column_name.clone(), seed.clone());

            warnings.push( format!(
                "You are about to alter the column `{column_name}` on the `A` table, which contains 1 non-null values. The data in that column will be cast from `{from}` to `{to}`.",
               column_name = column_name,
                from = with_default_params(from),
                to = with_default_params(to).to_uppercase(),
            ).into());

            previous_assertions.push((column_name.clone(), with_default_params(from).to_lowercase()).clone());
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
        api.assert_schema().await?.assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_column_count(previous_assertions.len() + 1),
                |acc, (column_name, expected)| {
                    acc.and_then(|table| table.assert_column(column_name, |c| c.assert_native_type(expected)))
                },
            )
        })?;

        let dm2 = api.native_types_datamodel(format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @test_db.Integer
                    {columns}
                }}
                "#,
            columns = next_columns
        ));

        api.schema_push(&dm2).send().await?.assert_warnings(&warnings)?;

        //second assertions same as first
        api.assert_schema().await?.assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_column_count(previous_assertions.len() + 1),
                |acc, (column_name, expected)| {
                    acc.and_then(|table| table.assert_column(column_name, |c| c.assert_native_type(expected)))
                },
            )
        })?;

        api.database()
            .raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()))
            .await?;
    }

    Ok(())
}

#[test_each_connector(tags("postgres"), features("native_types"))]
async fn not_castable_with_existing_data_should_warn(api: &TestApi) -> TestResult {
    for (from, seed, casts) in NOT_CASTABLE.iter() {
        let mut previous_columns = "".to_string();
        let mut next_columns = "".to_string();
        let mut insert = Insert::single_into((api.schema_name(), "A"));
        let mut previous_assertions = vec![];
        let mut warnings = vec![];

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

            insert = insert.value(column_name.clone(), seed.clone());

            warnings.push(
                format!(
                    "Changed the type of `{column_name}` on the `A` table.",
                    column_name = column_name,
                    // from = from,
                    // to = to,
                )
                .into(),
            );

            previous_assertions.push((column_name.clone(), with_default_params(from).to_lowercase()).clone());
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
        api.assert_schema().await?.assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_column_count(previous_assertions.len() + 1),
                |acc, (column_name, expected)| {
                    acc.and_then(|table| table.assert_column(column_name, |c| c.assert_native_type(expected)))
                },
            )
        })?;

        let dm2 = api.native_types_datamodel(format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @test_db.Integer
                    {columns}
                }}
                "#,
            columns = next_columns
        ));

        api.schema_push(&dm2).send().await?.assert_warnings(&warnings)?;

        //second assertions same as first
        api.assert_schema().await?.assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_column_count(previous_assertions.len() + 1),
                |acc, (column_name, expected)| {
                    acc.and_then(|table| table.assert_column(column_name, |c| c.assert_native_type(expected)))
                },
            )
        })?;

        api.database()
            .raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()))
            .await?;
    }

    Ok(())
}
