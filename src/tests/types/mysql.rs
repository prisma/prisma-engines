use crate::tests::test_api::*;
#[cfg(feature = "bigdecimal")]
use std::str::FromStr;

test_type!(tinyint(
    mysql,
    "tinyint(4)",
    Value::Integer(None),
    Value::integer(i8::MIN),
    Value::integer(i8::MAX)
));

test_type!(tinyint1(
    mysql,
    "tinyint(1)",
    Value::integer(-1),
    Value::integer(1),
    Value::integer(0)
));

test_type!(tinyint_unsigned(
    mysql,
    "tinyint(4) unsigned",
    Value::Integer(None),
    Value::integer(0),
    Value::integer(255)
));

test_type!(year(
    mysql,
    "year",
    Value::Integer(None),
    Value::integer(1984),
    Value::integer(2049)
));

test_type!(smallint(
    mysql,
    "smallint",
    Value::Integer(None),
    Value::integer(i16::MIN),
    Value::integer(i16::MAX)
));

test_type!(smallint_unsigned(
    mysql,
    "smallint unsigned",
    Value::Integer(None),
    Value::integer(0),
    Value::integer(65535)
));

test_type!(mediumint(
    mysql,
    "mediumint",
    Value::Integer(None),
    Value::integer(-8388608),
    Value::integer(8388607)
));

test_type!(mediumint_unsigned(
    mysql,
    "mediumint unsigned",
    Value::Integer(None),
    Value::integer(0),
    Value::integer(16777215)
));

test_type!(int(
    mysql,
    "int",
    Value::Integer(None),
    Value::integer(i32::MIN),
    Value::integer(i32::MAX)
));

test_type!(int_unsigned(
    mysql,
    "int unsigned",
    Value::Integer(None),
    Value::integer(0),
    Value::integer(4294967295i64)
));

test_type!(bigint(
    mysql,
    "bigint",
    Value::Integer(None),
    Value::integer(i64::MIN),
    Value::integer(i64::MAX)
));

#[cfg(feature = "bigdecimal")]
test_type!(decimal(
    mysql,
    "decimal(10,2)",
    Value::Numeric(None),
    Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap())
));

#[cfg(feature = "bigdecimal")]
test_type!(float_decimal(
    mysql,
    "float",
    (Value::Numeric(None), Value::Float(None)),
    (
        Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap()),
        Value::float(3.14)
    )
));

#[cfg(feature = "bigdecimal")]
test_type!(double_decimal(
    mysql,
    "double",
    (Value::Numeric(None), Value::Double(None)),
    (
        Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap()),
        Value::double(3.14)
    )
));

test_type!(bit1(
    mysql,
    "bit(1)",
    (Value::Bytes(None), Value::Boolean(None)),
    (Value::integer(0), Value::boolean(false)),
    (Value::integer(1), Value::boolean(true)),
));

test_type!(bit64(
    mysql,
    "bit(64)",
    Value::Bytes(None),
    Value::bytes(vec![0, 0, 0, 0, 0, 6, 107, 58])
));

test_type!(char(mysql, "char(255)", Value::Text(None), Value::text("foobar")));
test_type!(float(mysql, "float", Value::Float(None), Value::float(1.12345),));
test_type!(double(mysql, "double", Value::Double(None), Value::double(1.12314124)));
test_type!(varchar(mysql, "varchar(255)", Value::Text(None), Value::text("foobar")));
test_type!(tinytext(mysql, "tinytext", Value::Text(None), Value::text("foobar")));
test_type!(text(mysql, "text", Value::Text(None), Value::text("foobar")));
test_type!(longtext(mysql, "longtext", Value::Text(None), Value::text("foobar")));
test_type!(binary(mysql, "binary(5)", Value::bytes(vec![1, 2, 3, 0, 0])));
test_type!(varbinary(mysql, "varbinary(255)", Value::bytes(vec![1, 2, 3])));

test_type!(mediumtext(
    mysql,
    "mediumtext",
    Value::Text(None),
    Value::text("foobar")
));

test_type!(tinyblob(
    mysql,
    "tinyblob",
    Value::Bytes(None),
    Value::bytes(vec![1, 2, 3])
));

test_type!(mediumblob(
    mysql,
    "mediumblob",
    Value::Bytes(None),
    Value::bytes(vec![1, 2, 3])
));

test_type!(longblob(
    mysql,
    "longblob",
    Value::Bytes(None),
    Value::bytes(vec![1, 2, 3])
));

test_type!(blob(mysql, "blob", Value::Bytes(None), Value::bytes(vec![1, 2, 3])));

test_type!(enum(
    mysql,
    "enum('pollicle_dogs','jellicle_cats')",
    Value::Enum(None),
    Value::enum_variant("jellicle_cats"),
    Value::enum_variant("pollicle_dogs")
));

#[cfg(feature = "json")]
test_type!(json(
    mysql,
    "json",
    Value::Json(None),
    Value::json(serde_json::json!({"this": "is", "a": "json", "number": 2}))
));

#[cfg(feature = "chrono")]
test_type!(date(mysql, "date", Value::Date(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-04-20T00:00:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(time(
    mysql,
    "time",
    Value::Time(None),
    Value::time(chrono::NaiveTime::from_hms(16, 20, 00))
));

#[cfg(feature = "chrono")]
test_type!(datetime(mysql, "datetime", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(timestamp(mysql, "timestamp", {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));
