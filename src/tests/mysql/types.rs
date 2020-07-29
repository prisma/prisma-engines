use super::MySql;
use crate::tests::type_test::TypeTest;
use std::str::FromStr;

test_type!(tinyint(
    MySql,
    "tinyint(4)",
    Value::integer(i8::MIN),
    Value::integer(i8::MAX)
));

test_type!(year(MySql, "year", Value::integer(1984), Value::integer(2049)));

test_type!(smallint(
    MySql,
    "smallint(6)",
    Value::integer(i16::MIN),
    Value::integer(i16::MAX)
));

test_type!(int(
    MySql,
    "int(11)",
    Value::integer(i32::MIN),
    Value::integer(i32::MAX)
));

test_type!(bigint(
    MySql,
    "bigint(20)",
    Value::integer(i64::MIN),
    Value::integer(i64::MAX)
));

test_type!(decimal(
    MySql,
    "decimal(10,2)",
    Value::real(rust_decimal::Decimal::new(314, 2))
));

test_type!(float(
    MySql,
    "float",
    Value::real(rust_decimal::Decimal::from_str("1.1234").unwrap())
));

test_type!(double(
    MySql,
    "double",
    Value::real(rust_decimal::Decimal::from_str("1.12345").unwrap())
));

test_type!(bit64(MySql, "bit(64)", Value::bytes(vec![0, 0, 0, 0, 0, 6, 107, 58])));

// SQLx can get booleans here!
test_type!(boolean(MySql, "tinyint(1)", Value::integer(1), Value::integer(0)));

test_type!(char(MySql, "char(255)", Value::text("foobar")));
test_type!(varchar(MySql, "varchar(255)", Value::text("foobar")));
test_type!(tinytext(MySql, "tinytext", Value::text("foobar")));
test_type!(text(MySql, "text", Value::text("foobar")));
test_type!(mediumtext(MySql, "mediumtext", Value::text("foobar")));
test_type!(longtext(MySql, "longtext", Value::text("foobar")));

test_type!(binary(MySql, "binary(5)", Value::bytes(vec![1, 2, 3, 0, 0])));
test_type!(varbinary(MySql, "varbinary(255)", Value::bytes(vec![1, 2, 3])));
test_type!(tinyblob(MySql, "tinyblob", Value::bytes(vec![1, 2, 3])));
test_type!(mediumblob(MySql, "mediumblob", Value::bytes(vec![1, 2, 3])));
test_type!(blob(MySql, "blob", Value::bytes(vec![1, 2, 3])));
test_type!(longblob(MySql, "longblob", Value::bytes(vec![1, 2, 3])));

test_type!(enum(MySql, "enum('pollicle_dogs','jellicle_cats')", Value::text("jellicle_cats"), Value::text("pollicle_dogs")));

#[cfg(feature = "json-1")]
test_type!(json(
    MySql,
    "json",
    Value::json(serde_json::json!({"this": "is", "a": "json", "number": 2}))
));

#[cfg(feature = "chrono-0_4")]
test_type!(date(MySql, "date", {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-04-20T00:00:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono-0_4")]
test_type!(time(
    MySql,
    "time",
    Value::time(chrono::NaiveTime::from_hms(16, 20, 00))
));

#[cfg(feature = "chrono-0_4")]
test_type!(datetime(MySql, "datetime", {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono-0_4")]
test_type!(timestamp(MySql, "timestamp", {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));
