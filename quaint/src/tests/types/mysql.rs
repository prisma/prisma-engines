#![allow(clippy::approx_constant)]

use crate::tests::test_api::*;

#[cfg(feature = "bigdecimal")]
use std::str::FromStr;

#[cfg(feature = "bigdecimal")]
use crate::bigdecimal::BigDecimal;

test_type!(tinyint(
    mysql,
    "tinyint(4)",
    ValueInner::Int32(None),
    Value::int32(i8::MIN),
    Value::int32(i8::MAX)
));

test_type!(tinyint1(
    mysql,
    "tinyint(1)",
    Value::int32(-1),
    Value::int32(1),
    Value::int32(0)
));

test_type!(tinyint_unsigned(
    mysql,
    "tinyint(4) unsigned",
    ValueInner::Int32(None),
    Value::int32(0),
    Value::int32(255)
));

test_type!(year(
    mysql,
    "year",
    ValueInner::Int32(None),
    Value::int32(1984),
    Value::int32(2049)
));

test_type!(smallint(
    mysql,
    "smallint",
    ValueInner::Int32(None),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX)
));

test_type!(smallint_unsigned(
    mysql,
    "smallint unsigned",
    ValueInner::Int32(None),
    Value::int32(0),
    Value::int32(65535)
));

test_type!(mediumint(
    mysql,
    "mediumint",
    ValueInner::Int32(None),
    Value::int32(-8388608),
    Value::int32(8388607)
));

test_type!(mediumint_unsigned(
    mysql,
    "mediumint unsigned",
    ValueInner::Int64(None),
    Value::int64(0),
    Value::int64(16777215)
));

test_type!(int(
    mysql,
    "int",
    ValueInner::Int32(None),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX)
));

test_type!(int_unsigned(
    mysql,
    "int unsigned",
    ValueInner::Int64(None),
    Value::int64(0),
    Value::int64(2173158296i64),
    Value::int64(4294967295i64)
));

test_type!(int_unsigned_not_null(
    mysql,
    "int unsigned not null",
    Value::int64(0),
    Value::int64(2173158296i64),
    Value::int64(4294967295i64)
));

test_type!(bigint(
    mysql,
    "bigint",
    ValueInner::Int64(None),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX)
));

#[cfg(feature = "bigdecimal")]
test_type!(decimal(
    mysql,
    "decimal(10,2)",
    ValueInner::Numeric(None),
    Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap())
));

// Highest mantissa on MySQL
#[cfg(feature = "bigdecimal")]
test_type!(decimal_65_6(
    mysql,
    "decimal(65, 6)",
    Value::numeric(BigDecimal::from_str(
        "93431006223456789876545678909876545678903434334567834369999.345678"
    )?),
));

#[cfg(feature = "bigdecimal")]
test_type!(float_decimal(
    mysql,
    "float",
    (ValueInner::Numeric(None), ValueInner::Float(None)),
    (
        Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap()),
        Value::float(3.14)
    )
));

#[cfg(feature = "bigdecimal")]
test_type!(double_decimal(
    mysql,
    "double",
    (ValueInner::Numeric(None), ValueInner::Double(None)),
    (
        Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap()),
        Value::double(3.14)
    )
));

test_type!(bit1(
    mysql,
    "bit(1)",
    (ValueInner::Bytes(None), ValueInner::Boolean(None)),
    (Value::integer(0), Value::boolean(false)),
    (Value::integer(1), Value::boolean(true)),
));

test_type!(bit64(
    mysql,
    "bit(64)",
    ValueInner::Bytes(None),
    Value::bytes(vec![0, 0, 0, 0, 0, 6, 107, 58])
));

test_type!(char(mysql, "char(255)", ValueInner::Text(None), Value::text("foobar")));
test_type!(float(mysql, "float", ValueInner::Float(None), Value::float(1.12345),));
test_type!(double(
    mysql,
    "double",
    ValueInner::Double(None),
    Value::double(1.12314124)
));
test_type!(varchar(
    mysql,
    "varchar(255)",
    ValueInner::Text(None),
    Value::text("foobar")
));
test_type!(tinytext(
    mysql,
    "tinytext",
    ValueInner::Text(None),
    Value::text("foobar")
));
test_type!(text(mysql, "text", ValueInner::Text(None), Value::text("foobar")));
test_type!(longtext(
    mysql,
    "longtext",
    ValueInner::Text(None),
    Value::text("foobar")
));
test_type!(binary(mysql, "binary(5)", Value::bytes(vec![1, 2, 3, 0, 0])));
test_type!(varbinary(mysql, "varbinary(255)", Value::bytes(vec![1, 2, 3])));

test_type!(mediumtext(
    mysql,
    "mediumtext",
    ValueInner::Text(None),
    Value::text("foobar")
));

test_type!(tinyblob(
    mysql,
    "tinyblob",
    ValueInner::Bytes(None),
    Value::bytes(vec![1, 2, 3])
));

test_type!(mediumblob(
    mysql,
    "mediumblob",
    ValueInner::Bytes(None),
    Value::bytes(vec![1, 2, 3])
));

test_type!(longblob(
    mysql,
    "longblob",
    ValueInner::Bytes(None),
    Value::bytes(vec![1, 2, 3])
));

test_type!(blob(
    mysql,
    "blob",
    ValueInner::Bytes(None),
    Value::bytes(vec![1, 2, 3])
));

test_type!(enum(
    mysql,
    "enum('pollicle_dogs','jellicle_cats')",
    ValueInner::Enum(None, None),
    Value::enum_variant("jellicle_cats"),
    Value::enum_variant("pollicle_dogs")
));

test_type!(json(
    mysql,
    "json",
    ValueInner::Json(None),
    Value::json(serde_json::json!({"this": "is", "a": "json", "number": 2}))
));

test_type!(date(mysql, "date", ValueInner::Date(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-04-20T00:00:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(time(
    mysql,
    "time",
    ValueInner::Time(None),
    Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap())
));

test_type!(datetime(mysql, "datetime", ValueInner::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(timestamp(mysql, "timestamp", {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));
