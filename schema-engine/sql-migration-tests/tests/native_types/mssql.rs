use bigdecimal::BigDecimal;
use chrono::Utc;
use once_cell::sync::Lazy;
use quaint::{prelude::Insert, Value};
use sql_migration_tests::test_api::*;
use std::{collections::HashMap, str::FromStr};

static SAFE_CASTS: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        (
            "Bit",
            Value::boolean(true),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(32,16)",
                "Money",
                "SmallMoney",
                "Float(24)",
                "Float(53)",
                "Real",
                "DateTime",
                "SmallDateTime",
                "Binary",
                "Binary(10)",
                "VarBinary",
                "VarBinary(10)",
                "VarBinary(Max)",
                "Char",
                "Char(10)",
                "NChar",
                "NChar(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "NVarChar",
                "NVarChar(10)",
                "NVarChar(Max)",
            ],
        ),
        (
            "TinyInt",
            Value::int32(u8::MAX),
            &[
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,0)",
                "Money",
                "SmallMoney",
                "Float(24)",
                "Float(53)",
                "Real",
                "DateTime",
                "SmallDateTime",
                "Binary",
                "Binary(10)",
                "VarBinary",
                "VarBinary(10)",
                "Char(3)",
                "NChar(3)",
                "VarChar(3)",
                "NVarChar(3)",
            ],
        ),
        (
            "SmallInt",
            Value::int32(i16::MAX),
            &[
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(5,0)",
                "Money",
                "SmallMoney",
                "Float(24)",
                "Float(53)",
                "Real",
                "DateTime",
                "SmallDateTime",
                "Binary(2)",
                "VarBinary(2)",
                "Char(6)",
                "NChar(6)",
                "VarChar(6)",
                "NVarChar(6)",
            ],
        ),
        (
            "Int",
            Value::int32(i32::MAX),
            &[
                "BigInt",
                "Decimal",
                "Decimal(10,0)",
                "Money",
                "Float(24)",
                "Float(53)",
                "Real",
                "Binary(4)",
                "VarBinary(4)",
                "Char(11)",
                "NChar(11)",
                "VarChar(11)",
                "NVarChar(11)",
            ],
        ),
        (
            "BigInt",
            Value::int64(i64::MAX),
            &[
                "Decimal(19,0)",
                "Float(24)",
                "Float(53)",
                "Real",
                "Binary(8)",
                "VarBinary(8)",
                "Char(20)",
                "NChar(20)",
                "VarChar(20)",
                "NVarChar(20)",
            ],
        ),
        (
            "Decimal",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &[
                "Decimal",
                "Decimal(18,0)",
                "Decimal(20,2)",
                "Char(19)",
                "NChar(19)",
                "VarChar(19)",
                "NVarChar(19)",
            ],
        ),
        (
            "Decimal(4,2)",
            Value::numeric(BigDecimal::from_str("12.32").unwrap()),
            &["Decimal(5,3)", "Char(6)", "NChar(6)", "VarChar(6)", "NVarChar(6)"],
        ),
        (
            "Money",
            Value::numeric(BigDecimal::from_str("420.6666").unwrap()),
            &[
                "Decimal(19,4)",
                "Char(21)",
                "NChar(21)",
                "VarChar(21)",
                "NVarChar(21)",
                "Binary(8)",
                "VarBinary(8)",
            ],
        ),
        (
            "SmallMoney",
            Value::numeric(BigDecimal::from_str("420.6666").unwrap()),
            &[
                "Decimal(10,4)",
                "Char(12)",
                "NChar(12)",
                "VarChar(12)",
                "NVarChar(12)",
                "Binary(4)",
                "VarBinary(4)",
                "Money",
            ],
        ),
        (
            "Float",
            Value::float(1.23),
            &[
                "Float(53)",
                "Char(317)",
                "NChar(317)",
                "VarChar(317)",
                "NVarChar(317)",
                "Binary(8)",
                "VarBinary(8)",
            ],
        ),
        (
            "Real",
            Value::double(1.23),
            &[
                "Float(24)",
                "Char(47)",
                "NChar(47)",
                "VarChar(47)",
                "NVarChar(47)",
                "Binary(4)",
                "VarBinary(4)",
            ],
        ),
        (
            "Date",
            Value::date(Utc::now().naive_utc().date()),
            &[
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "Char(10)",
                "NChar(10)",
                "VarChar(10)",
                "NVarChar(10)",
            ],
        ),
        (
            "Time",
            Value::time(Utc::now().naive_utc().time()),
            &[
                "DateTime2",
                "DateTimeOffset",
                "Char(8)",
                "NChar(8)",
                "VarChar(8)",
                "NVarChar(8)",
            ],
        ),
        (
            "DateTime",
            Value::datetime(Utc::now()),
            &[
                "DateTime2",
                "DateTimeOffset",
                "Char(23)",
                "NChar(23)",
                "VarChar(23)",
                "NVarChar(23)",
            ],
        ),
        (
            "DateTime2",
            Value::datetime(Utc::now()),
            &["DateTimeOffset", "Char(27)", "NChar(27)", "VarChar(27)", "NVarChar(27)"],
        ),
        (
            "DateTimeOffset",
            Value::datetime(Utc::now()),
            &["Char(33)", "NChar(33)", "VarChar(33)", "NVarChar(33)"],
        ),
        (
            "SmallDateTime",
            Value::datetime(Utc::now()),
            &[
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "Char(19)",
                "NChar(19)",
                "VarChar(19)",
                "NVarChar(19)",
            ],
        ),
        (
            "Char",
            Value::text("f"),
            &[
                "Char(10)",
                "NChar",
                "NChar(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "NVarChar",
                "NVarChar(10)",
                "NVarChar(Max)",
                "Text",
                "NText",
            ],
        ),
        (
            "NChar",
            Value::text("f"),
            &["NChar(10)", "NVarChar", "NVarChar(10)", "NVarChar(Max)", "NText"],
        ),
        (
            "Char(10)",
            Value::text("foo"),
            &["Char(20)", "NChar(10)", "NVarChar(10)", "NVarChar(Max)", "NText"],
        ),
        (
            "NChar(10)",
            Value::text("foo"),
            &["NChar(20)", "NVarChar(10)", "NVarChar(Max)", "NText"],
        ),
        (
            "VarChar",
            Value::text("f"),
            &[
                "Char",
                "Char(10)",
                "VarChar",
                "VarChar(10)",
                "NChar",
                "NChar(10)",
                "NVarChar",
                "NVarChar(10)",
                "NVarChar(Max)",
                "Text",
                "NText",
            ],
        ),
        (
            "VarChar(10)",
            Value::text("foo"),
            &[
                "Char(10)",
                "VarChar(11)",
                "NChar(10)",
                "NVarChar(10)",
                "NVarChar(Max)",
                "Text",
                "NText",
            ],
        ),
        ("VarChar(Max)", Value::text("foo"), &["Text"]),
        ("NVarChar(Max)", Value::text("foo"), &["NText"]),
        (
            "NVarChar",
            Value::text("f"),
            &["NChar", "NChar(10)", "NVarChar(10)", "NVarChar(Max)", "NText"],
        ),
        ("Text", Value::text("foo"), &["VarChar(Max)"]),
        ("NText", Value::text("foo"), &["NVarChar(Max)"]),
        (
            "Binary",
            Value::bytes(vec![1]),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Money",
                "SmallMoney",
                "Char",
                "NChar",
                "VarChar",
                "NVarChar",
                "Binary(2)",
                "VarBinary",
                "VarBinary(10)",
                "Image",
            ],
        ),
        (
            "Binary(16)",
            Value::bytes(vec![1, 2, 3]),
            &[
                "Char(16)",
                "NChar(8)",
                "VarChar(16)",
                "NVarChar(8)",
                "Binary(32)",
                "VarBinary(16)",
                "Image",
            ],
        ),
        (
            "VarBinary",
            Value::bytes(vec![1]),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Money",
                "SmallMoney",
                "Char",
                "NChar",
                "VarChar",
                "NVarChar",
                "Binary",
                "VarBinary(10)",
                "VarBinary(Max)",
                "Image",
            ],
        ),
        (
            "VarBinary(16)",
            Value::bytes(vec![1, 2, 3]),
            &[
                "Char(16)",
                "NChar(8)",
                "VarChar(16)",
                "NVarChar(8)",
                "Binary(16)",
                "VarBinary(32)",
                "VarBinary(Max)",
                "Image",
            ],
        ),
        (
            "VarBinary(Max)",
            Value::bytes(vec![1, 2, 3]),
            &["VarChar(Max)", "NVarChar(Max)", "Image"],
        ),
        ("Image", Value::bytes(vec![1, 2, 3]), &["VarBinary(Max)"]),
        ("Xml", Value::text("<jamon>iberico</jamon>"), &["NVarChar(Max)"]),
        (
            "UniqueIdentifier",
            Value::text("75bf0037-a8b8-4512-beea-5a186f8abf1e"),
            &[
                "Char(36)",
                "NChar(36)",
                "VarChar(36)",
                "VarChar(Max)",
                "NVarChar(36)",
                "NVarChar(Max)",
                "Binary(16)",
                "VarBinary(16)",
                "VarBinary(Max)",
            ],
        ),
    ]
});

static RISKY_CASTS: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        (
            "TinyInt",
            Value::int32(u8::MAX),
            &["Decimal(2,0)", "Char(2)", "NChar(2)", "VarChar(2)", "NVarChar(2)"],
        ),
        (
            "SmallInt",
            Value::int32(i16::MAX),
            &[
                "Bit",
                "TinyInt",
                "Decimal(4,0)",
                "Char(5)",
                "NChar(5)",
                "VarChar(5)",
                "NVarChar(5)",
                "Binary",
                "VarBinary",
            ],
        ),
        (
            "Int",
            Value::int32(i32::MAX),
            &[
                "Bit",
                "TinyInt",
                "SmallInt",
                "SmallMoney",
                "DateTime",
                "SmallDateTime",
                "Decimal(9,0)",
                "Char",
                "Char(10)",
                "NChar",
                "NChar(10)",
                "VarChar",
                "VarChar(10)",
                "NVarChar",
                "NVarChar(10)",
                "Binary",
                "Binary(3)",
                "VarBinary",
                "VarBinary(3)",
            ],
        ),
        (
            "BigInt",
            Value::int32(i32::MAX),
            &[
                "Bit",
                "TinyInt",
                "SmallInt",
                "Int",
                "Money",
                "SmallMoney",
                "DateTime",
                "SmallDateTime",
                "Decimal",
                "Decimal(9,0)",
                "Char",
                "Char(19)",
                "NChar",
                "NChar(19)",
                "VarChar",
                "VarChar(19)",
                "NVarChar",
                "NVarChar(19)",
                "Binary",
                "Binary(7)",
                "VarBinary",
                "VarBinary(7)",
            ],
        ),
        (
            "Decimal(3,2)",
            Value::numeric(BigDecimal::from_str("1.23").unwrap()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "DateTime",
                "SmallDateTime",
                "Binary",
                "VarBinary",
                "Decimal(2,2)",
                "Decimal(3,1)",
                "Decimal",
                "Char(3)",
                "Char",
                "NChar(3)",
                "NChar",
                "VarChar(3)",
                "NVarChar(3)",
                "NVarChar",
            ],
        ),
        (
            "Decimal",
            Value::numeric(BigDecimal::from_str("123123").unwrap()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "DateTime",
                "SmallDateTime",
                "Binary",
                "VarBinary",
                "Decimal(18,1)",
                "Decimal(17,0)",
                "Char(18)",
                "Char",
                "NChar(18)",
                "NChar",
                "VarChar(18)",
                "NVarChar(18)",
                "NVarChar",
            ],
        ),
        (
            "Money",
            Value::numeric(BigDecimal::from_str("12.3456").unwrap()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "DateTime",
                "SmallDateTime",
                "Binary(7)",
                "Binary",
                "VarBinary",
                "VarBinary(7)",
                "Decimal(18,3)",
                "Decimal",
                "Char(20)",
                "Char",
                "NChar(20)",
                "NChar",
                "VarChar(20)",
                "NVarChar(20)",
                "NVarChar",
            ],
        ),
        (
            "SmallMoney",
            Value::numeric(BigDecimal::from_str("12.3456").unwrap()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Bit",
                "Float",
                "Real",
                "DateTime",
                "SmallDateTime",
                "Binary(3)",
                "Binary",
                "VarBinary",
                "VarBinary(3)",
                "Decimal(9,3)",
                "Decimal",
                "Char(11)",
                "Char",
                "NChar(11)",
                "NChar",
                "VarChar(11)",
                "NVarChar(11)",
                "NVarChar",
            ],
        ),
        (
            "Float(24)",
            Value::float(1.23),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Bit",
                "Float",
                "Float(53)",
                "Money",
                "SmallMoney",
                "DateTime",
                "SmallDateTime",
                "Binary",
                "Binary(3)",
                "VarBinary",
                "VarBinary(3)",
                "Decimal(9,3)",
                "Decimal",
                "Char(46)",
                "Char",
                "NChar(46)",
                "NChar",
                "VarChar(46)",
                "NVarChar(46)",
                "NVarChar",
            ],
        ),
        (
            "Real",
            Value::float(1.23),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Bit",
                "Float",
                "Float(53)",
                "Money",
                "SmallMoney",
                "DateTime",
                "SmallDateTime",
                "Binary",
                "Binary(3)",
                "VarBinary",
                "VarBinary(3)",
                "Decimal(9,3)",
                "Decimal",
                "Char(46)",
                "Char",
                "NChar(46)",
                "NChar",
                "VarChar(46)",
                "NVarChar(46)",
                "NVarChar",
            ],
        ),
        (
            "Float(53)",
            Value::double(1.23),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Bit",
                "Real",
                "Float(24)",
                "Money",
                "SmallMoney",
                "DateTime",
                "SmallDateTime",
                "Binary",
                "Binary(7)",
                "VarBinary",
                "VarBinary(7)",
                "Decimal(9,3)",
                "Decimal",
                "Char(316)",
                "Char",
                "NChar(316)",
                "NChar",
                "VarChar(316)",
                "NVarChar(316)",
                "NVarChar",
            ],
        ),
        (
            "Float",
            Value::double(1.23),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Bit",
                "Real",
                "Float(24)",
                "Money",
                "SmallMoney",
                "DateTime",
                "SmallDateTime",
                "Binary",
                "Binary(7)",
                "VarBinary",
                "VarBinary(7)",
                "Decimal(9,3)",
                "Decimal",
                "Char(316)",
                "Char",
                "NChar(316)",
                "NChar",
                "VarChar(316)",
                "NVarChar(316)",
                "NVarChar",
            ],
        ),
        ("Date", Value::date(Utc::now().naive_utc().date()), &["SmallDateTime"]),
        (
            "Time",
            Value::time(Utc::now().naive_utc().time()),
            &[
                "SmallDateTime",
                "DateTime",
                "Char",
                "Char(7)",
                "NChar",
                "NChar(7)",
                "VarChar",
                "VarChar(7)",
                "NVarChar",
                "NVarChar(7)",
            ],
        ),
        (
            "DateTime",
            Value::datetime(Utc::now()),
            &[
                "Date",
                "Time",
                "SmallDateTime",
                "Char",
                "Char(22)",
                "NChar",
                "NChar(22)",
                "VarChar",
                "VarChar(22)",
                "NVarChar",
                "NVarChar(22)",
            ],
        ),
        (
            "DateTime2",
            Value::datetime(Utc::now()),
            &[
                "DateTime",
                "Date",
                "Time",
                "SmallDateTime",
                "Char",
                "Char(26)",
                "NChar",
                "NChar(26)",
                "VarChar",
                "VarChar(26)",
                "NVarChar",
                "NVarChar(26)",
            ],
        ),
        (
            "DateTimeOffset",
            Value::datetime(Utc::now()),
            &[
                "DateTime",
                "DateTime2",
                "Date",
                "Time",
                "SmallDateTime",
                "Char",
                "Char(32)",
                "NChar",
                "NChar(32)",
                "VarChar",
                "VarChar(32)",
                "NVarChar",
                "NVarChar(32)",
            ],
        ),
        (
            "SmallDateTime",
            Value::datetime(Utc::now()),
            &[
                "Date",
                "Time",
                "Char",
                "Char(18)",
                "NChar",
                "NChar(18)",
                "VarChar",
                "VarChar(18)",
                "NVarChar",
                "NVarChar(18)",
            ],
        ),
        (
            "Char",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "UniqueIdentifier",
            ],
        ),
        (
            "Char(10)",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(9)",
                "NChar",
                "VarChar",
                "VarChar(9)",
                "NVarChar",
                "NVarChar(9)",
                "UniqueIdentifier",
            ],
        ),
        (
            "NChar",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "Text",
                "UniqueIdentifier",
            ],
        ),
        (
            "NChar(10)",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "NChar",
                "NChar(9)",
                "NVarChar(9)",
                "Text",
                "UniqueIdentifier",
            ],
        ),
        (
            "VarChar",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "UniqueIdentifier",
            ],
        ),
        (
            "VarChar(10)",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(9)",
                "NChar",
                "NChar(9)",
                "VarChar",
                "VarChar(9)",
                "NVarChar",
                "NVarChar(9)",
                "UniqueIdentifier",
            ],
        ),
        (
            "VarChar(Max)",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(9)",
                "NChar",
                "NChar(9)",
                "VarChar",
                "VarChar(9)",
                "NVarChar",
                "NVarChar(9)",
                "NVarChar(Max)",
                "NText",
                "UniqueIdentifier",
            ],
        ),
        (
            "NChar",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "Text",
                "UniqueIdentifier",
            ],
        ),
        (
            "NChar(10)",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "NChar",
                "NChar(9)",
                "NVarChar(9)",
                "Text",
                "UniqueIdentifier",
            ],
        ),
        (
            "NVarChar",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "Text",
                "UniqueIdentifier",
            ],
        ),
        (
            "NVarChar(10)",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(9)",
                "NChar",
                "NChar(9)",
                "VarChar",
                "VarChar(9)",
                "NVarChar",
                "NVarChar(9)",
                "VarChar",
                "VarChar(9)",
                "VarChar(Max)",
                "NChar",
                "NChar(9)",
                "Text",
                "UniqueIdentifier",
            ],
        ),
        (
            "NVarChar(Max)",
            Value::text("f"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,2)",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Float(53)",
                "Real",
                "Float(24)",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "Char(9)",
                "NChar",
                "NChar(9)",
                "VarChar",
                "VarChar(9)",
                "VarChar(Max)",
                "NVarChar",
                "NVarChar(9)",
                "Text",
                "UniqueIdentifier",
            ],
        ),
        (
            "Text",
            Value::text("f"),
            &[
                "Char",
                "Char(10)",
                "NChar",
                "NChar(10)",
                "VarChar",
                "VarChar(10)",
                "NVarChar",
                "NVarChar(10)",
                "NVarChar(Max)",
                "NText",
            ],
        ),
        (
            "NText",
            Value::text("f"),
            &[
                "Char",
                "Char(10)",
                "NChar",
                "NChar(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "NVarChar",
                "NVarChar(10)",
                "Text",
            ],
        ),
        (
            "Binary",
            Value::bytes(vec![1]),
            &[
                "Bit",
                "Decimal",
                "Decimal(3,1)",
                "DateTime",
                "SmallDateTime",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Binary(10)",
            Value::bytes(vec![1, 2, 3]),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Decimal(3,1)",
                "Bit",
                "Char",
                "Char(9)",
                "NChar",
                "NChar(4)",
                "VarChar",
                "VarChar(9)",
                "NVarChar",
                "NVarChar(4)",
                "Binary",
                "VarBinary",
                "Binary(9)",
                "VarBinary(9)",
                "DateTime",
                "SmallDateTime",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "VarBinary",
            Value::bytes(vec![1]),
            &[
                "Bit",
                "Decimal",
                "Decimal(3,1)",
                "DateTime",
                "SmallDateTime",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "VarBinary(10)",
            Value::bytes(vec![1, 2, 3]),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Decimal(3,1)",
                "Bit",
                "Char",
                "Char(9)",
                "NChar",
                "NChar(4)",
                "VarChar",
                "VarChar(9)",
                "NVarChar",
                "NVarChar(4)",
                "Binary",
                "VarBinary",
                "Binary(9)",
                "VarBinary(9)",
                "DateTime",
                "SmallDateTime",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "VarBinary(Max)",
            Value::bytes(vec![1, 2, 3]),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Decimal(3,1)",
                "Bit",
                "Char",
                "Char(9)",
                "NChar",
                "NChar(4)",
                "VarChar",
                "VarChar(9)",
                "NVarChar",
                "NVarChar(4)",
                "Binary",
                "VarBinary",
                "Binary(9)",
                "VarBinary(9)",
                "DateTime",
                "SmallDateTime",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Image",
            Value::bytes(vec![1, 2, 3]),
            &["VarBinary(10)", "VarBinary", "Binary", "Binary(10)"],
        ),
        (
            "Xml",
            Value::text("<move>johnny</move>"),
            &[
                "Char",
                "Char(10)",
                "NChar",
                "NChar(10)",
                "VarChar",
                "VarChar(10)",
                "VarChar(Max)",
                "NVarChar",
                "NVarChar(10)",
            ],
        ),
    ]
});

static NOT_CASTABLE: Lazy<Vec<(&str, Value, &[&str])>> = Lazy::new(|| {
    vec![
        (
            "Bit",
            Value::boolean(false),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "UniqueIdentifier",
            ],
        ),
        (
            "TinyInt",
            Value::int32(u8::MAX),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "SmallInt",
            Value::int32(i16::MAX),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Int",
            Value::int32(i32::MAX),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "BigInt",
            Value::int64(i64::MAX),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Decimal",
            Value::numeric(BigDecimal::from_str("1").unwrap()),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Money",
            Value::numeric(BigDecimal::from_str("420.6666").unwrap()),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "SmallMoney",
            Value::numeric(BigDecimal::from_str("420.6666").unwrap()),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Float",
            Value::float(1.23),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Real",
            Value::double(1.23),
            &[
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Date",
            Value::date(Utc::now().naive_utc().date()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Time",
                "Text",
                "NText",
                "Binary",
                "VarBinary",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Time",
            Value::time(Utc::now().naive_utc().time()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Date",
                "Text",
                "NText",
                "Binary",
                "VarBinary",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "DateTime",
            Value::datetime(Utc::now()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Text",
                "NText",
                "Binary",
                "VarBinary",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "DateTime2",
            Value::datetime(Utc::now()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Text",
                "NText",
                "Binary",
                "VarBinary",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "DateTimeOffset",
            Value::datetime(Utc::now()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Text",
                "NText",
                "Binary",
                "VarBinary",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "SmallDateTime",
            Value::datetime(Utc::now()),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Text",
                "NText",
                "Binary",
                "VarBinary",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        ("Char", Value::text("f"), &["Binary", "VarBinary", "Image", "Xml"]),
        ("NChar", Value::text("f"), &["Binary", "VarBinary", "Image", "Xml"]),
        ("VarChar", Value::text("f"), &["Binary", "VarBinary", "Image", "Xml"]),
        (
            "Text",
            Value::text("foo"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Binary",
                "VarBinary",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "NText",
            Value::text("foo"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Binary",
                "VarBinary",
                "Image",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Binary",
            Value::bytes(vec![1]),
            &[
                "Float",
                "Real",
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
            ],
        ),
        (
            "VarBinary",
            Value::bytes(vec![1]),
            &[
                "Float",
                "Real",
                "Date",
                "Time",
                "DateTime2",
                "DateTimeOffset",
                "Text",
                "NText",
            ],
        ),
        (
            "Image",
            Value::bytes(vec![1, 2, 3]),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Char",
                "NChar",
                "VarChar",
                "Text",
                "NVarChar",
                "NText",
                "Xml",
                "UniqueIdentifier",
            ],
        ),
        (
            "Xml",
            Value::text("<jamon>iberico</jamon>"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Text",
                "NText",
                "UniqueIdentifier",
            ],
        ),
        (
            "UniqueIdentifier",
            Value::text("ce6ae17b-d877-4dbf-85cf-fd0daf0c1385"),
            &[
                "TinyInt",
                "SmallInt",
                "Int",
                "BigInt",
                "Decimal",
                "Money",
                "SmallMoney",
                "Bit",
                "Float",
                "Real",
                "Date",
                "Time",
                "DateTime",
                "DateTime2",
                "DateTimeOffset",
                "SmallDateTime",
                "Text",
                "NText",
                "Image",
                "Xml",
            ],
        ),
    ]
});

static TYPE_MAPS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut maps = HashMap::new();

    maps.insert("TinyInt", "Int");
    maps.insert("SmallInt", "Int");
    maps.insert("Int", "Int");
    maps.insert("BigInt", "BigInt");
    maps.insert("Decimal", "Decimal");
    maps.insert("Money", "Float");
    maps.insert("SmallMoney", "Float");
    maps.insert("SmallMoney", "Float");
    maps.insert("Bit", "Boolean");
    maps.insert("Float", "Float");
    maps.insert("Real", "Float");
    maps.insert("Date", "DateTime");
    maps.insert("Time", "DateTime");
    maps.insert("DateTime", "DateTime");
    maps.insert("DateTime2", "DateTime");
    maps.insert("DateTimeOffset", "DateTime");
    maps.insert("SmallDateTime", "DateTime");
    maps.insert("Char", "String");
    maps.insert("NChar", "String");
    maps.insert("VarChar", "String");
    maps.insert("Text", "String");
    maps.insert("NVarChar", "String");
    maps.insert("NText", "String");
    maps.insert("Binary", "Bytes");
    maps.insert("VarBinary", "Bytes");
    maps.insert("Image", "Bytes");
    maps.insert("Xml", "String");
    maps.insert("UniqueIdentifier", "String");

    maps
});

fn with_params(r#type: &str) -> &str {
    match r#type {
        "Decimal" => "Decimal(18,0)",
        "Float(24)" => "Real",
        "Float" => "Float(53)",
        "Binary" => "Binary(1)",
        "VarBinary" => "VarBinary(1)",
        "VarChar" => "VarChar(1)",
        "NVarChar" => "NVarChar(1)",
        "Char" => "Char(1)",
        "NChar" => "NChar(1)",
        _ => r#type,
    }
}

#[test_connector(tags(Mssql))]
fn safe_casts_with_existing_data_should_work(api: TestApi) {
    for (from, seed, casts) in SAFE_CASTS.iter() {
        for to in *casts {
            println!("From `{from}` to `{to}` with seed `{seed:?}`");

            let kind = from.split('(').next().unwrap();

            let dm1 = &format!(
                r#"
               model A {{
                    id Int @id @default(autoincrement()) @db.Int
                    x  {} @db.{}
                }}
                "#,
                TYPE_MAPS.get(kind).unwrap(),
                from,
            );

            api.schema_push_w_datasource(dm1).send().assert_green();

            let insert = Insert::single_into((api.schema_name(), "A".to_owned())).value("x", seed.clone());
            api.query(insert.into());

            api.assert_schema().assert_table("A", |table| {
                table.assert_columns_count(2).assert_column("x", |c| {
                    c.assert_is_required()
                        .assert_full_data_type(&with_params(from).to_lowercase())
                })
            });

            let kind = to.split('(').next().unwrap();

            let dm2 = &format!(
                r#"
               model A {{
                    id Int @id @default(autoincrement()) @db.Int
                    x  {} @db.{}
                }}
                "#,
                TYPE_MAPS.get(kind).unwrap(),
                to,
            );

            api.schema_push_w_datasource(dm2).send().assert_green();

            api.assert_schema().assert_table("A", |table| {
                table.assert_columns_count(2).assert_column("x", |c| {
                    c.assert_is_required()
                        .assert_full_data_type(&with_params(to).to_lowercase())
                })
            });

            api.raw_cmd(&format!("DROP TABLE [{}].[A]", api.schema_name()));
        }
    }
}

#[test_connector(tags(Mssql))]
fn risky_casts_with_existing_data_should_warn(api: TestApi) {
    for (from, seed, casts) in RISKY_CASTS.iter() {
        for to in *casts {
            println!("From `{from}` to `{to}` with seed `{seed:?}`");

            let kind = from.split('(').next().unwrap();

            let dm1 = &format!(
                r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Int
                    x  {} @db.{}
                }}
                "#,
                TYPE_MAPS.get(kind).unwrap(),
                from,
            );

            api.schema_push_w_datasource(dm1).send().assert_green();

            let insert = Insert::single_into((api.schema_name(), "A".to_owned())).value("x", seed.clone());
            api.query(insert.into());

            api.assert_schema().assert_table("A", |table| {
                table.assert_columns_count(2).assert_column("x", |c| {
                    c.assert_is_required()
                        .assert_full_data_type(&with_params(from).to_lowercase())
                })
            });

            let kind = to.split('(').next().unwrap();

            let dm2 = &format!(
                r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Int
                    x  {} @db.{}
                }}
                "#,
                TYPE_MAPS.get(kind).unwrap(),
                to
            );

            let warning = format!(
                "You are about to alter the column `x` on the `A` table, which contains 1 non-null values. The data in that column will be cast from `{}` to `{}`.",
                with_params(from),
                to,
            );

            api.schema_push_w_datasource(dm2)
                .send()
                .assert_warnings(&[warning.into()]);

            api.assert_schema().assert_table("A", |table| {
                table.assert_columns_count(2).assert_column("x", |c| {
                    c.assert_is_required()
                        .assert_full_data_type(&with_params(from).to_lowercase())
                })
            });

            api.raw_cmd(&format!("DROP TABLE [{}].[A]", api.schema_name()));
        }
    }
}

#[test_connector(tags(Mssql))]
fn not_castable_with_existing_data_should_warn(api: TestApi) {
    for (from, seed, casts) in NOT_CASTABLE.iter() {
        for to in *casts {
            println!("From `{from}` to `{to}` with seed `{seed:?}`");

            let kind = match from.split('(').next() {
                Some(a) => a,
                _ => unreachable!(),
            };

            let dm1 = &format!(
                r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Int
                    x  {} @db.{}
                }}
                "#,
                TYPE_MAPS.get(kind).unwrap(),
                from,
            );

            api.schema_push_w_datasource(dm1).send().assert_green();

            let insert = Insert::single_into((api.schema_name(), "A".to_owned())).value("x", seed.clone());
            api.query(insert.into());

            api.assert_schema().assert_table("A", |table| {
                table.assert_columns_count(2).assert_column("x", |c| {
                    c.assert_is_required()
                        .assert_full_data_type(&with_params(from).to_lowercase())
                })
            });

            let kind = to.split('(').next().unwrap();

            let dm2 = &format!(
                r#"
                model A {{
                    id Int @id @default(autoincrement()) @db.Int
                    x  {} @db.{}
                }}
                "#,
                TYPE_MAPS.get(kind).unwrap(),
                to
            );

            let warning = "Changed the type of `x` on the `A` table. No cast exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table.";

            api.schema_push_w_datasource(dm2)
                .send()
                .assert_unexecutable(&[warning.into()]);

            api.assert_schema().assert_table("A", |table| {
                table.assert_columns_count(2).assert_column("x", |c| {
                    c.assert_is_required()
                        .assert_full_data_type(&with_params(from).to_lowercase())
                })
            });

            api.raw_cmd(&format!("DROP TABLE [{}].[A]", api.schema_name()));
        }
    }
}

#[test_connector(tags(Mssql))]
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
            id        Int     @id @default(autoincrement()) @db.Int
            title     String  @db.NVarChar(1000)
            content   String? @db.NVarChar(1000)
            published Boolean @default(false) @db.Bit
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?    @db.Int
        }

        model User {
            id    Int     @id @default(autoincrement()) @db.Int
            email String  @unique @db.NVarChar(1000)
            name  String? @db.NVarChar(1000)
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

#[test_connector(tags(Mssql))]
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
            id        Int     @id @default(autoincrement()) @db.Int
            title     String  @db.NVarChar(1000)
            content   String? @db.NVarChar(MAX)
            published Boolean @default(false) @db.Bit
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?    @db.Int
        }

        model User {
            id    Int     @id @default(autoincrement()) @db.Int
            email String  @unique @db.NVarChar(1000)
            name  String? @db.NVarChar(100)
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
