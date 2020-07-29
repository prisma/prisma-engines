use super::MsSql;
use crate::tests::type_test::TypeTest;
use std::str::FromStr;

test_type!(nvarchar_limited(
    MsSql,
    "NVARCHAR(10)",
    Value::Text(None),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(nvarchar_max(
    MsSql,
    "NVARCHAR(max)",
    Value::Text(None),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(ntext(
    MsSql,
    "NTEXT",
    Value::Text(None),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(varchar_limited(
    MsSql,
    "VARCHAR(10)",
    Value::Text(None),
    Value::text("foobar"),
));

test_type!(varchar_max(
    MsSql,
    "VARCHAR(max)",
    Value::Text(None),
    Value::text("foobar"),
));

test_type!(text(MsSql, "TEXT", Value::Text(None), Value::text("foobar")));

test_type!(tinyint(
    MsSql,
    "tinyint",
    Value::Integer(None),
    Value::integer(u8::MIN),
    Value::integer(u8::MAX),
));

test_type!(smallint(
    MsSql,
    "smallint",
    Value::Integer(None),
    Value::integer(i16::MIN),
    Value::integer(i16::MAX),
));

test_type!(int(
    MsSql,
    "int",
    Value::Integer(None),
    Value::integer(i32::MIN),
    Value::integer(i32::MAX),
));

test_type!(bigint(
    MsSql,
    "bigint",
    Value::Integer(None),
    Value::integer(i64::MIN),
    Value::integer(i64::MAX),
));

test_type!(decimal(
    MsSql,
    "decimal(10,2)",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::new(314, 2))
));

test_type!(numeric(
    MsSql,
    "numeric(10,2)",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::new(314, 2))
));

test_type!(money(
    MsSql,
    "money",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::new(314, 2))
));

test_type!(smallmoney(
    MsSql,
    "smallmoney",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::new(314, 2))
));

test_type!(float_24(
    MsSql,
    "float(24)",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::from_str("1.1234567").unwrap())
));

test_type!(real(
    MsSql,
    "real",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::from_str("1.1234567").unwrap())
));

test_type!(float_53(
    MsSql,
    "float(53)",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::from_str("1.123456789012345").unwrap())
));

test_type!(boolean(
    MsSql,
    "bit",
    Value::Boolean(None),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(binary(
    MsSql,
    "binary(8)",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(varbinary(
    MsSql,
    "varbinary(8)",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(image(
    MsSql,
    "image",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec()),
));

#[cfg(feature = "chrono-0_4")]
test_type!(date(
    MsSql,
    "date",
    Value::Date(None),
    Value::date(chrono::NaiveDate::from_ymd(2020, 4, 20))
));

#[cfg(feature = "chrono-0_4")]
test_type!(time(
    MsSql,
    "time",
    Value::Time(None),
    Value::time(chrono::NaiveTime::from_hms(16, 20, 00))
));

#[cfg(feature = "chrono-0_4")]
test_type!(datetime2(MsSql, "datetime2", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono-0_4")]
test_type!(datetime(MsSql, "datetime", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono-0_4")]
test_type!(datetimeoffset(MsSql, "datetimeoffset", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono-0_4")]
test_type!(smalldatetime(MsSql, "smalldatetime", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));
