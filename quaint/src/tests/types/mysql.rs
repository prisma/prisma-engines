#![allow(clippy::approx_constant)]

use std::str::FromStr;

use crate::bigdecimal::BigDecimal;
use crate::macros::assert_matching_value_and_column_type;
use crate::{connector::ColumnType, tests::test_api::*};

test_type!(tinyint(
    mysql,
    "tinyint(4)",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i8::MIN),
    Value::int32(i8::MAX)
));

test_type!(tinyint1(
    mysql,
    "tinyint(1)",
    ColumnType::Int32,
    Value::int32(-1),
    Value::int32(1),
    Value::int32(0)
));

test_type!(tinyint_unsigned(
    mysql,
    "tinyint(4) unsigned",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(0),
    Value::int32(255)
));

test_type!(year(
    mysql,
    "year",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(1984),
    Value::int32(2049)
));

test_type!(smallint(
    mysql,
    "smallint",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX)
));

test_type!(smallint_unsigned(
    mysql,
    "smallint unsigned",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(0),
    Value::int32(65535)
));

test_type!(mediumint(
    mysql,
    "mediumint",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(-8388608),
    Value::int32(8388607)
));

test_type!(mediumint_unsigned(
    mysql,
    "mediumint unsigned",
    ColumnType::Int64,
    Value::null_int64(),
    Value::int64(0),
    Value::int64(16777215)
));

test_type!(int(
    mysql,
    "int",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX)
));

test_type!(int_unsigned(
    mysql,
    "int unsigned",
    ColumnType::Int64,
    Value::null_int64(),
    Value::int64(0),
    Value::int64(2173158296i64),
    Value::int64(4294967295i64)
));

test_type!(int_unsigned_not_null(
    mysql,
    "int unsigned not null",
    ColumnType::Int64,
    Value::int64(0),
    Value::int64(2173158296i64),
    Value::int64(4294967295i64)
));

test_type!(bigint(
    mysql,
    "bigint",
    ColumnType::Int64,
    Value::null_int64(),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX)
));

test_type!(decimal(
    mysql,
    "decimal(10,2)",
    ColumnType::Numeric,
    Value::null_numeric(),
    Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap())
));

// Highest mantissa on MySQL
test_type!(decimal_65_6(
    mysql,
    "decimal(65, 6)",
    ColumnType::Numeric,
    Value::numeric(BigDecimal::from_str(
        "93431006223456789876545678909876545678903434334567834369999.345678"
    )?),
));

test_type!(float_decimal(
    mysql,
    "float",
    ColumnType::Float,
    (Value::null_numeric(), Value::null_float()),
    (
        Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap()),
        Value::float(3.14)
    )
));

test_type!(double_decimal(
    mysql,
    "double",
    ColumnType::Double,
    (Value::null_numeric(), Value::null_double()),
    (
        Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap()),
        Value::double(3.14)
    )
));

test_type!(bit1(
    mysql,
    "bit(1)",
    ColumnType::Boolean,
    (Value::null_bytes(), Value::null_boolean()),
    (Value::int32(0), Value::boolean(false)),
    (Value::int32(1), Value::boolean(true)),
));

test_type!(bit64(
    mysql,
    "bit(64)",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(vec![0, 0, 0, 0, 0, 6, 107, 58])
));

test_type!(char(
    mysql,
    "char(255)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));
test_type!(float(
    mysql,
    "float",
    ColumnType::Float,
    Value::null_float(),
    Value::float(1.12345),
));
test_type!(double(
    mysql,
    "double",
    ColumnType::Double,
    Value::null_double(),
    Value::double(1.12314124)
));
test_type!(varchar(
    mysql,
    "varchar(255)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));
test_type!(tinytext(
    mysql,
    "tinytext",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));
test_type!(text(
    mysql,
    "text",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));
test_type!(longtext(
    mysql,
    "longtext",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));
test_type!(binary(
    mysql,
    "binary(5)",
    ColumnType::Bytes,
    Value::bytes(vec![1, 2, 3, 0, 0])
));
test_type!(varbinary(
    mysql,
    "varbinary(255)",
    ColumnType::Bytes,
    Value::bytes(vec![1, 2, 3])
));

test_type!(mediumtext(
    mysql,
    "mediumtext",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));

test_type!(tinyblob(
    mysql,
    "tinyblob",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(vec![1, 2, 3])
));

test_type!(mediumblob(
    mysql,
    "mediumblob",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(vec![1, 2, 3])
));

test_type!(longblob(
    mysql,
    "longblob",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(vec![1, 2, 3])
));

test_type!(blob(
    mysql,
    "blob",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(vec![1, 2, 3])
));

test_type!(enum(
    mysql,
    "enum('pollicle_dogs','jellicle_cats')",
    ColumnType::Enum,
    Value::null_enum(),
    Value::enum_variant("jellicle_cats"),
    Value::enum_variant("pollicle_dogs")
));

test_type!(json(
    mysql,
    "json",
    ColumnType::Json,
    Value::null_json(),
    Value::json(serde_json::json!({"this": "is", "a": "json", "number": 2}))
));

test_type!(date(
    mysql,
    "date",
    ColumnType::Date,
    (Value::null_date(), Value::null_date()),
    (
        Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap()),
        Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap())
    ),
    (
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2020-04-20T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        ),
        Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap())
    )
));

test_type!(time(
    mysql,
    "time",
    ColumnType::Time,
    Value::null_time(),
    Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap())
));

test_type!(datetime(
    mysql,
    "datetime",
    ColumnType::DateTime,
    Value::null_datetime(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
        Value::datetime(dt.with_timezone(&chrono::Utc))
    }
));

test_type!(timestamp(mysql, "timestamp", ColumnType::DateTime, {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));
