use bigdecimal::BigDecimal;
use chrono::Utc;
use once_cell::sync::Lazy;
use quaint::{prelude::Insert, Value};
use sql_migration_tests::test_api::*;
use std::{collections::HashMap, fmt::Write as _, str::FromStr};

static SAFE_CASTS: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        ("Oid", Value::int32(u8::MAX), &["VarChar(100)", "Integer", "BigInt"]),
        ("Money", Value::int64(u8::MAX), &["VarChar(100)"]),
        ("Inet", Value::text("10.1.2.3"), &["VarChar(100)"]),
        (
            "SmallInt",
            Value::int32(u8::MAX),
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
            Value::int32(i32::MAX),
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
            Value::int64(i64::MAX),
            &["BigInt", "Real", "DoublePrecision", "VarChar(53)", "Char(53)", "Text"],
        ),
        (
            "Decimal(10,2)",
            Value::numeric(BigDecimal::from_str("12345678.90").unwrap()),
            &["Decimal(32,16)", "VarChar(53)", "Char(53)", "Text"],
        ),
        (
            "Decimal(2,0)",
            Value::numeric(BigDecimal::from_str("12").unwrap()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "VarChar(53)",
                "Char(53)",
                "Text",
            ],
        ),
        (
            "Real",
            Value::float(f32::MIN),
            &["DoublePrecision", "VarChar(53)", "VarChar", "Char(53)", "Text"],
        ),
        (
            "DoublePrecision",
            Value::double(f64::MIN),
            &["DoublePrecision", "Text", "VarChar", "Char(1000)"],
        ),
        ("VarChar", Value::text("fiver"), &["Text"]),
        ("VarChar(5)", Value::text("fiver"), &["VarChar(53)", "Char(53)", "Text"]),
        (
            "Char(1)", // same as Char
            Value::text("t"),
            &["VarChar(3)", "Char(3)", "Text"],
        ),
        ("Text", Value::text("true"), &["VarChar", "Text"]),
        ("ByteA", Value::bytes(b"DEAD".to_vec()), &["Text", "VarChar"]),
        (
            "Timestamp(3)",
            Value::datetime(Utc::now()),
            &[
                "VarChar(23)",
                "Char(23)",
                "Text",
                "Timestamp(1)",
                "Timestamptz(3)",
                "Date",
                "Time(3)",
            ],
        ),
        (
            "Timestamptz(3)",
            Value::datetime(Utc::now()),
            &[
                "VarChar(28)",
                "Char(53)",
                "Text",
                "Timestamp(1)",
                "Date",
                "Time(3)",
                "Timetz(3)",
            ],
        ),
        (
            "Date",
            Value::date(Utc::now().naive_utc().date()),
            &["VarChar(53)", "Char(28)", "Text", "Timestamp(3)", "Timestamptz(3)"],
        ),
        (
            "Time(3)",
            Value::time(Utc::now().naive_utc().time()),
            &["VarChar(14)", "Char(53)", "Text", "Timetz(3)"],
        ),
        (
            "Timetz(3)",
            Value::datetime(Utc::now()),
            &["VarChar(53)", "Char(19)", "Text", "Time(3)", "Timetz(6)"],
        ),
        ("Boolean", Value::boolean(false), &["VarChar", "Char(5)", "Text"]),
        (
            "Bit(1)", // same as Bit
            Value::text("0"),
            &["VarChar", "Char(1)", "Char(5)", "Text", "VarBit(10)"],
        ),
        (
            "Bit(10)",
            Value::text("0010101001"),
            &["VarChar(53)", "Char(53)", "Text", "VarBit(10)"],
        ),
        ("VarBit", Value::text("000101010101010010"), &["VarChar", "Text"]),
        ("VarBit(5)", Value::text("0010"), &["VarChar(53)", "Char(53)", "Text"]),
        (
            "Uuid",
            Value::text("75bf0037-a8b8-4512-beea-5a186f8abf1e"),
            &["VarChar(53)", "Char(53)", "Text"],
        ),
        ("Xml", Value::xml("[]"), &["VarChar", "Text"]),
        (
            "Json",
            Value::json(serde_json::json!({"foo": "bar"})),
            &["Text", "JsonB", "VarChar"],
        ),
        (
            "JsonB",
            Value::json(serde_json::json!({"foo": "bar"})),
            &["Text", "Json", "VarChar"],
        ),
    ]
});

static RISKY_CASTS: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        ("Money", Value::int64(u8::MAX), &["Decimal"]),
        ("SmallInt", Value::int32(2), &["Decimal(2,1)", "VarChar(3)", "Char(1)"]),
        ("Integer", Value::int32(1), &["Decimal(2,1)", "VarChar(4)", "Char(1)"]),
        ("BigInt", Value::int32(2), &["Decimal(2,1)", "VarChar(17)", "Char(1)"]),
        (
            "Decimal(10,2)",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Real",
                "DoublePrecision",
                "VarChar(5)",
                "Char(5)",
            ],
        ),
        (
            "Decimal(5,0)",
            Value::numeric(BigDecimal::from_str("10").unwrap()),
            &["SmallInt", "VarChar(5)", "Char(5)", "Real", "DoublePrecision"],
        ),
        (
            "Real",
            Value::float(3.0),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "VarChar(10)",
                "Char(1)",
            ],
        ),
        (
            "DoublePrecision",
            Value::double(3.0),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "VarChar(5)",
                "Char(1)",
            ],
        ),
        ("VarChar(5)", Value::text("t"), &["VarChar(3)", "Char(1)"]),
        ("Text", Value::text("t"), &["VarChar(3)", "Char(1)"]),
        ("ByteA", Value::bytes(vec![1]), &["VarChar(4)", "Char(5)"]),
        ("VarBit(5)", Value::text("001"), &["Bit(3)"]),
        ("Xml", Value::xml("[]"), &["VarChar(100)", "Char(100)"]),
        (
            "Json",
            Value::json(serde_json::json!({"foo": "bar"})),
            &["VarChar(100)", "Char(100)"],
        ),
        (
            "JsonB",
            Value::json(serde_json::json!({"foo": "bar"})),
            &["VarChar(100)", "Char(100)"],
        ),
    ]
});

static NOT_CASTABLE: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        (
            "SmallInt",
            Value::int32(u8::MAX),
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
            Value::int32(i32::MAX),
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
            Value::int64(i64::MAX),
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
            "Decimal(5,0)",
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
            Value::double(7.5),
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
            "VarChar(5)",
            Value::text("true"),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
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
            "Char(5)",
            Value::text("true"),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
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
            "Text",
            Value::text("true"),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
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
            "ByteA",
            Value::bytes(vec![1]),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
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
            "Timestamp(3)",
            Value::datetime(Utc::now()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
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
            "Timestamptz(3)",
            Value::datetime(Utc::now()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
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
            "Date",
            Value::date(Utc::now().naive_utc().date()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
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
            "Time(3)",
            Value::time(Utc::now().naive_utc().time()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
                "Timestamp(3)",
                "Timestamptz(3)",
                "Date",
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
            "Timetz(3)",
            Value::datetime(Utc::now()),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
                "Timestamp(3)",
                "Timestamptz(3)",
                "Date",
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
            "Boolean",
            Value::boolean(true),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
                "Timestamp(3)",
                "Timestamptz(3)",
                "Date",
                "Time(3)",
                "Timetz(3)",
                "Bit(10)",
                "VarBit(10)",
                "Uuid",
                "Xml",
                "Json",
                "JsonB",
            ],
        ),
        (
            "Bit(10)",
            Value::text("0010101001"),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
                "Timestamp(3)",
                "Timestamptz(3)",
                "Date",
                "Time(3)",
                "Timetz(3)",
                "Boolean",
                "Uuid",
                "Xml",
                "Json",
                "JsonB",
            ],
        ),
        (
            "VarBit(5)",
            Value::text("0010"),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
                "Timestamp(3)",
                "Timestamptz(3)",
                "Date",
                "Time(3)",
                "Timetz(3)",
                "Boolean",
                "Bit(10)",
                "Uuid",
                "Xml",
                "Json",
                "JsonB",
            ],
        ),
        (
            "Uuid",
            Value::text("75bf0037-a8b8-4512-beea-5a186f8abf1e"),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
                "ByteA",
                "Timestamp(3)",
                "Timestamptz(3)",
                "Date",
                "Time(3)",
                "Timetz(3)",
                "Boolean",
                "Bit(10)",
                "VarBit(10)",
                "Xml",
                "Json",
                "JsonB",
            ],
        ),
        (
            "Xml",
            Value::text("[]"),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
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
                "Json",
                "JsonB",
            ],
        ),
        (
            "Json",
            Value::json(serde_json::json!({"foo": "bar"})),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
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
            ],
        ),
        (
            "JsonB",
            Value::json(serde_json::json!({"foo": "bar"})),
            &[
                "SmallInt",
                "Integer",
                "BigInt",
                "Decimal(32,16)",
                "Real",
                "DoublePrecision",
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
            ],
        ),
    ]
});

static TYPE_MAPS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut maps = HashMap::new();

    maps.insert("SmallInt", "Int");
    maps.insert("Integer", "Int");
    maps.insert("BigInt", "BigInt");
    maps.insert("Decimal", "Decimal");
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
    maps.insert("Oid", "Int");
    maps.insert("Money", "Decimal");
    maps.insert("Inet", "String");

    maps
});

fn prisma_type(native_type: &str) -> &str {
    let kind = native_type.split('(').next().unwrap();
    TYPE_MAPS.get(kind).unwrap()
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn safe_casts_with_existing_data_should_work(api: TestApi) {
    let connector = psl::builtin_connectors::POSTGRES;

    for (from, seed, casts) in SAFE_CASTS.iter() {
        let span = tracing::info_span!("SafeCasts", from = %from, to = ?casts, seed = ?seed);
        let _span = span.enter();

        let mut previous_columns = "".to_string();
        let mut next_columns = "".to_string();
        let mut insert = Insert::single_into((api.schema_name(), "A".to_owned()));
        let mut previous_assertions = vec![];
        let mut next_assertions = vec![];

        for (idx, to) in casts.iter().enumerate() {
            println!("From `{from}` to `{to}` with seed `{seed:?}`");

            let column_name = format!("column_{idx}");

            writeln!(
                previous_columns,
                "{column_name}  {prisma_type}? @db.{native_type}",
                prisma_type = prisma_type(from),
                native_type = from,
                column_name = column_name
            )
            .unwrap();

            writeln!(
                next_columns,
                "{column_name}  {prisma_type}? @db.{native_type}",
                prisma_type = prisma_type(to),
                native_type = to,
                column_name = column_name
            )
            .unwrap();

            insert = insert.value(column_name.clone(), seed.clone());

            previous_assertions.push((column_name.clone(), *from));
            next_assertions.push((column_name, to).clone());
        }

        let dm1 = format!(
            r#"
               model A {{
                    id Int @id @default(autoincrement()) @db.Integer
                    {previous_columns}
                }}
                "#,
        );

        tracing::info!(dm = dm1.as_str());

        api.schema_push_w_datasource(&dm1).send().assert_green();

        // inserts
        api.query(insert.into());

        // first assertions
        api.assert_schema().assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_columns_count(previous_assertions.len() + 1),
                |table, (column_name, expected)| {
                    table.assert_column(column_name, |c| c.assert_native_type(expected, connector))
                },
            )
        });

        let dm2 = format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Integer
                    {next_columns}
                }}
                "#
        );

        api.schema_push_w_datasource(&dm2).send().assert_green();

        // second assertions
        api.assert_schema().assert_table("A", |table| {
            next_assertions.iter().fold(
                table.assert_columns_count(next_assertions.len() + 1),
                |table, (name, expected)| table.assert_column(name, |c| c.assert_native_type(expected, connector)),
            )
        });

        api.raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()));
    }
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn risky_casts_with_existing_data_should_warn(api: TestApi) {
    let connector = psl::builtin_connectors::POSTGRES;

    for (from, seed, casts) in RISKY_CASTS.iter() {
        let mut previous_columns = "".to_string();
        let mut next_columns = "".to_string();
        let mut insert = Insert::single_into((api.schema_name(), "A".to_owned()));
        let mut previous_assertions = vec![];
        let mut next_assertions = vec![];
        let mut warnings = vec![];

        for (idx, to) in casts.iter().enumerate() {
            println!("From `{from}` to `{to}` with seed `{seed:?}`");

            let column_name = format!("column_{idx}");

            writeln!(
                previous_columns,
                "{column_name}  {prisma_type}? @db.{native_type}",
                prisma_type = prisma_type(from),
                native_type = from,
                column_name = column_name
            )
            .unwrap();

            writeln!(
                next_columns,
                "{column_name}  {prisma_type}? @db.{native_type}",
                prisma_type = prisma_type(to),
                native_type = to,
                column_name = column_name
            )
            .unwrap();

            insert = insert.value(column_name.clone(), seed.clone());

            warnings.push( format!(
                "You are about to alter the column `{column_name}` on the `A` table, which contains 1 non-null values. The data in that column will be cast from `{from}` to `{to}`.",
            ).into());

            previous_assertions.push((column_name.clone(), *from));
            next_assertions.push((column_name.clone(), *to));
        }

        let dm1 = format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Integer
                    {previous_columns}
                }}
                "#,
        );

        api.schema_push_w_datasource(&dm1).send().assert_green();

        // inserts
        api.query(insert.into());

        // first assertions

        api.assert_schema().assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_columns_count(previous_assertions.len() + 1),
                |table, (column_name, expected)| {
                    table.assert_column(column_name, |c| c.assert_native_type(expected, connector))
                },
            )
        });

        let dm2 = format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Integer
                    {next_columns}
                }}
                "#,
        );

        api.schema_push_w_datasource(&dm2)
            .force(true)
            .send()
            .assert_warnings(&warnings);

        //second assertions same as first
        api.assert_schema().assert_table("A", |table| {
            next_assertions.iter().fold(
                table.assert_columns_count(next_assertions.len() + 1),
                |table, (column_name, expected)| {
                    table.assert_column(column_name, |c| c.assert_native_type(expected, connector))
                },
            )
        });

        api.raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()));
    }
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn not_castable_with_existing_data_should_warn(api: TestApi) {
    let connector = psl::builtin_connectors::POSTGRES;
    let mut warnings = Vec::new();

    for (from, seed, casts) in NOT_CASTABLE.iter() {
        let mut previous_columns = "".to_string();
        let mut next_columns = "".to_string();
        let mut insert = Insert::single_into((api.schema_name(), "A".to_owned()));
        let mut previous_assertions = vec![];
        warnings.clear();

        for (idx, to) in casts.iter().enumerate() {
            println!("From `{from}` to `{to}` with seed `{seed:?}`");

            let column_name = format!("column_{idx}");

            writeln!(
                previous_columns,
                "{column_name}  {prisma_type}? @db.{native_type}",
                prisma_type = prisma_type(from),
                native_type = from,
                column_name = column_name
            )
            .unwrap();

            writeln!(
                next_columns,
                "{column_name}  {prisma_type}? @db.{native_type}",
                prisma_type = prisma_type(to),
                native_type = to,
                column_name = column_name
            )
            .unwrap();

            insert = insert.value(column_name.clone(), seed.clone());

            //todo adjust to mention the to and from
            warnings.push(
                format!(
                    "The `{column_name}` column on the `A` table would be dropped and recreated. This will lead to data loss.",
                    // from = from,
                    // to = to,
                )
                .into(),
            );

            previous_assertions.push((column_name, from).clone());
        }

        let dm1 = format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Integer
                    {previous_columns}
                }}
                "#,
        );

        api.schema_push_w_datasource(&dm1).send().assert_green();

        // inserts
        api.query(insert.into());

        // first assertions
        api.assert_schema().assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_columns_count(previous_assertions.len() + 1),
                |table, (column_name, expected)| {
                    table.assert_column(column_name, |c| c.assert_native_type(expected, connector))
                },
            )
        });

        let dm2 = format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Integer
                    {next_columns}
                }}
                "#,
        );

        // todo we could force here and then check that the db really returns not castable
        // then we would again need to have separate calls per mapping
        api.schema_push_w_datasource(&dm2).send().assert_warnings(&warnings);

        //second assertions same as first
        api.assert_schema().assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_columns_count(previous_assertions.len() + 1),
                |table, (column_name, expected)| {
                    table.assert_column(column_name, |c| c.assert_native_type(expected, connector))
                },
            )
        });

        api.raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()));
    }
}

/// A list of casts which can safely be performed.
type CastList = Lazy<Vec<(&'static str, Vec<(&'static str, Value<'static>)>)>>;

static SAFE_CASTS_NON_LIST_TO_STRING: CastList = Lazy::new(|| {
    vec![
        (
            "Text",
            vec![
                ("SmallInt", Value::array(vec![1])),
                ("Integer", Value::array(vec![Value::int32(i32::MAX)])),
                ("BigInt", Value::array(vec![Value::int64(i64::MAX)])),
                (
                    "Decimal(10,2)",
                    Value::array(vec![Value::numeric(BigDecimal::from_str("128.90").unwrap())]),
                ),
                ("Real", Value::array(vec![Value::float(f32::MIN)])),
                ("DoublePrecision", Value::array(vec![Value::double(f64::MIN)])),
                ("VarChar", Value::array(vec!["test"])),
                ("Char(1)", Value::array(vec!["a"])),
                ("Text", Value::array(vec!["text"])),
                ("ByteA", Value::array(vec![Value::bytes(b"DEAD".to_vec())])),
                ("Timestamp(3)", Value::array(vec![Value::datetime(Utc::now())])),
                ("Timestamptz(3)", Value::array(vec![Value::datetime(Utc::now())])),
                ("Date", Value::array(vec![Value::date(Utc::now().naive_utc().date())])),
                (
                    "Time(3)",
                    Value::array(vec![Value::time(Utc::now().naive_utc().time())]),
                ),
                ("Timetz(3)", Value::array(vec![Value::datetime(Utc::now())])),
                ("Boolean", Value::array(vec![false])),
                ("Bit(10)", Value::array(vec![Value::text("0010101001")])),
                ("VarBit", Value::array(vec![Value::text("000101010101010010")])),
                ("Uuid", Value::array(vec!["75bf0037-a8b8-4512-beea-5a186f8abf1e"])),
                ("Xml", Value::array(vec![Value::xml("[]")])),
                (
                    "Json",
                    Value::array(vec![Value::json(serde_json::json!({"foo": "bar"}))]),
                ),
                (
                    "JsonB",
                    Value::array(vec![Value::json(serde_json::json!({"foo": "bar"}))]),
                ),
            ],
        ),
        (
            "VarChar",
            vec![
                ("SmallInt", Value::array(vec![1])),
                ("Integer", Value::array(vec![Value::int32(i32::MAX)])),
                ("BigInt", Value::array(vec![Value::int64(i64::MAX)])),
                (
                    "Decimal(10,2)",
                    Value::array(vec![Value::numeric(BigDecimal::from_str("128.90").unwrap())]),
                ),
                ("Real", Value::array(vec![Value::float(f32::MIN)])),
                ("DoublePrecision", Value::array(vec![Value::double(f64::MIN)])),
                ("VarChar", Value::array(vec!["test"])),
                ("Char(1)", Value::array(vec!["a"])),
                ("Text", Value::array(vec!["text"])),
                ("ByteA", Value::array(vec![Value::bytes(b"DEAD".to_vec())])),
                ("Timestamp(3)", Value::array(vec![Value::datetime(Utc::now())])),
                ("Timestamptz(3)", Value::array(vec![Value::datetime(Utc::now())])),
                ("Date", Value::array(vec![Value::date(Utc::now().naive_utc().date())])),
                (
                    "Time(3)",
                    Value::array(vec![Value::time(Utc::now().naive_utc().time())]),
                ),
                ("Timetz(3)", Value::array(vec![Value::datetime(Utc::now())])),
                ("Boolean", Value::array(vec![false])),
                ("Bit(10)", Value::array(vec![Value::text("0010101001")])),
                ("VarBit", Value::array(vec![Value::text("000101010101010010")])),
                ("Uuid", Value::array(vec!["75bf0037-a8b8-4512-beea-5a186f8abf1e"])),
                ("Xml", Value::array(vec![Value::xml("[]")])),
                (
                    "Json",
                    Value::array(vec![Value::json(serde_json::json!({"foo": "bar"}))]),
                ),
                (
                    "JsonB",
                    Value::array(vec![Value::json(serde_json::json!({"foo": "bar"}))]),
                ),
            ],
        ),
    ]
});

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn safe_casts_from_array_with_existing_data_should_work(api: TestApi) {
    let connector = psl::builtin_connectors::POSTGRES;

    for (to, from) in SAFE_CASTS_NON_LIST_TO_STRING.iter() {
        let mut previous_columns = "".to_string();
        let mut next_columns = "".to_string();
        let mut insert = Insert::single_into((api.schema_name(), "A".to_owned()));
        let mut previous_assertions = vec![];
        let mut next_assertions = vec![];

        for (idx, (from, seed)) in from.iter().enumerate() {
            println!("From `{from}` to `{to}` with seed `{seed:?}`");

            let column_name = format!("column_{idx}");

            writeln!(
                previous_columns,
                "{column_name}  {prisma_type}[] @db.{native_type}",
                prisma_type = prisma_type(from),
                native_type = from,
                column_name = column_name
            )
            .unwrap();

            writeln!(
                next_columns,
                "{column_name}  {prisma_type} @db.{native_type}",
                prisma_type = prisma_type(to),
                native_type = to,
                column_name = column_name
            )
            .unwrap();

            insert = insert.value(column_name.clone(), seed.clone());

            previous_assertions.push((column_name.clone(), from));
            next_assertions.push((column_name, to).clone());
        }

        let dm1 = format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Integer
                    {previous_columns}
                }}
                "#,
        );

        api.schema_push_w_datasource(&dm1).send().assert_green();

        // inserts
        api.query(insert.into());

        // first assertions
        api.assert_schema().assert_table("A", |table| {
            previous_assertions.iter().fold(
                table.assert_columns_count(previous_assertions.len() + 1),
                |table, (column_name, expected)| {
                    table.assert_column(column_name, |c| c.assert_native_type(expected, connector))
                },
            )
        });

        let dm2 = format!(
            r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Integer
                    {next_columns}
                }}
                "#,
        );

        api.schema_push_w_datasource(&dm2).send().assert_green();

        //second assertions
        api.assert_schema().assert_table("A", |table| {
            next_assertions.iter().fold(
                table.assert_columns_count(next_assertions.len() + 1),
                |table, (name, expected)| table.assert_column(name, |c| c.assert_native_type(expected, connector)),
            )
        });

        api.raw_cmd(&format!("DROP TABLE \"{}\".\"A\"", api.schema_name()));
    }
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn typescript_starter_schema_with_native_types_is_idempotent(api: TestApi) {
    let dm = r#"
        model Post {
            id        Int     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?
        }

        model User {
            id    Int     @id @default(autoincrement())
            email String  @unique
            name  String?
            posts Post[]
        }
    "#;

    let dm2 = r#"
        model Post {
            id        Int     @id @default(autoincrement()) @db.Integer
            title     String  @db.Text
            content   String? @db.Text
            published Boolean @default(false) @db.Boolean
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?    @db.Integer
        }

        model User {
            id    Int     @id @default(autoincrement()) @db.Integer
            email String  @unique @db.Text
            name  String? @db.Text
            posts Post[]
        }
    "#;

    api.schema_push_w_datasource(dm)
        .migration_id(Some("first"))
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm)
        .migration_id(Some("second"))
        .send()
        .assert_green()
        .assert_no_steps();
    api.schema_push_w_datasource(dm2)
        .migration_id(Some("third"))
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn typescript_starter_schema_with_different_native_types_is_idempotent(api: TestApi) {
    let dm = r#"
        model Post {
            id        Int     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?
        }

        model User {
            id    Int     @id @default(autoincrement())
            email String  @unique
            name  String?
            posts Post[]
        }
    "#;

    let dm2 = r#"
        model Post {
            id        Int     @id @default(autoincrement()) @db.Integer
            title     String  @db.VarChar(100)
            content   String? @db.VarChar(100)
            published Boolean @default(false) @db.Boolean
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?    @db.Integer
        }

        model User {
            id    Int     @id @default(autoincrement()) @db.Integer
            email String  @unique @db.VarChar(100)
            name  String? @db.VarChar(100)
            posts Post[]
        }
    "#;

    api.schema_push_w_datasource(dm)
        .migration_id(Some("first"))
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm)
        .migration_id(Some("second"))
        .send()
        .assert_green()
        .assert_no_steps();
    api.schema_push_w_datasource(dm2)
        .migration_id(Some("third"))
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm2)
        .migration_id(Some("fourth"))
        .send()
        .assert_green()
        .assert_no_steps();
}
