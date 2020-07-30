use crate::tests::connector::Sqlite;
use std::str::FromStr;

test_type!(integer(
    Sqlite,
    "INTEGER",
    Value::Integer(None),
    Value::integer(i8::MIN),
    Value::integer(i8::MAX),
    Value::integer(i16::MIN),
    Value::integer(i16::MAX),
    Value::integer(i32::MIN),
    Value::integer(i32::MAX),
    Value::integer(i64::MIN),
    Value::integer(i64::MAX)
));

test_type!(real(
    Sqlite,
    "REAL",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::from_str("1.12345").unwrap())
));

test_type!(text(Sqlite, "TEXT", Value::Text(None), Value::text("foobar huhuu")));

test_type!(blob(
    Sqlite,
    "BLOB",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec())
));
test_type!(boolean(
    Sqlite,
    "BOOLEAN",
    Value::Boolean(None),
    Value::boolean(true),
    Value::boolean(false)
));

#[cfg(feature = "chrono-0_4")]
test_type!(date(
    Sqlite,
    "DATE",
    Value::Date(None),
    Value::date(chrono::NaiveDate::from_ymd(1984, 1, 1))
));

#[cfg(feature = "chrono-0_4")]
test_type!(datetime(
    Sqlite,
    "DATETIME",
    Value::DateTime(None),
    Value::datetime(chrono::DateTime::from_str("2020-07-29T09:23:44.458Z").unwrap())
));
