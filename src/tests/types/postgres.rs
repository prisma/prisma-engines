#[cfg(feature = "bigdecimal")]
mod bigdecimal;

use crate::tests::test_api::*;
use std::str::FromStr;

test_type!(boolean(
    postgres,
    "boolean",
    Value::Boolean(None),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(boolean_array(
    postgres,
    "boolean[]",
    Value::Array(None),
    Value::array(vec![true, false, true]),
));

test_type!(int2(
    postgres,
    "int2",
    Value::Integer(None),
    Value::integer(i16::MIN),
    Value::integer(i16::MAX),
));

test_type!(int2_array(
    postgres,
    "int2[]",
    Value::Array(None),
    Value::array(vec![1, 2, 3]),
));

test_type!(int4(
    postgres,
    "int4",
    Value::Integer(None),
    Value::integer(i32::MIN),
    Value::integer(i32::MAX),
));

test_type!(int4_array(
    postgres,
    "int4[]",
    Value::Array(None),
    Value::array(vec![1, 2, 3]),
));

test_type!(int8(
    postgres,
    "int8",
    Value::Integer(None),
    Value::integer(i64::MIN),
    Value::integer(i64::MAX),
));

test_type!(int8_array(
    postgres,
    "int8[]",
    Value::Array(None),
    Value::array(vec![1, 2, 3]),
));

test_type!(oid(postgres, "oid", Value::Integer(None), Value::integer(10000)));

test_type!(oid_array(
    postgres,
    "oid[]",
    Value::Array(None),
    Value::array(vec![1, 2, 3]),
));

test_type!(serial2(
    postgres,
    "serial2",
    Value::integer(i16::MIN),
    Value::integer(i16::MAX),
));

test_type!(serial4(
    postgres,
    "serial4",
    Value::integer(i32::MIN),
    Value::integer(i32::MAX),
));

test_type!(serial8(
    postgres,
    "serial8",
    Value::integer(i64::MIN),
    Value::integer(i64::MAX),
));

test_type!(char(postgres, "char(6)", Value::Text(None), Value::text("foobar")));

test_type!(char_array(
    postgres,
    "char(6)[]",
    Value::Array(None),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf")])
));

test_type!(varchar(
    postgres,
    "varchar(255)",
    Value::Text(None),
    Value::text("foobar")
));

test_type!(varchar_array(
    postgres,
    "varchar(255)[]",
    Value::Array(None),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf")])
));

test_type!(text(postgres, "text", Value::Text(None), Value::text("foobar")));

test_type!(text_array(
    postgres,
    "text[]",
    Value::Array(None),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf")])
));

test_type!(bit(postgres, "bit(4)", Value::Text(None), Value::text("1001")));

test_type!(bit_array(
    postgres,
    "bit(4)[]",
    Value::Array(None),
    Value::array(vec![Value::text("1001"), Value::text("0110")])
));

test_type!(varbit(
    postgres,
    "varbit(20)",
    Value::Text(None),
    Value::text("001010101")
));

test_type!(varbit_array(
    postgres,
    "varbit(20)[]",
    Value::Array(None),
    Value::array(vec![Value::text("001010101"), Value::text("01101111")])
));

test_type!(inet(postgres, "inet", Value::Text(None), Value::text("127.0.0.1")));

test_type!(inet_array(
    postgres,
    "inet[]",
    Value::Array(None),
    Value::array(vec![Value::text("127.0.0.1"), Value::text("192.168.1.1")])
));

#[cfg(feature = "json")]
test_type!(json(
    postgres,
    "json",
    Value::Json(None),
    Value::json(serde_json::json!({"foo": "bar"}))
));

#[cfg(feature = "json")]
test_type!(json_array(
    postgres,
    "json[]",
    Value::Array(None),
    Value::array(vec![
        serde_json::json!({"foo": "bar"}),
        serde_json::json!({"omg": false})
    ])
));

#[cfg(feature = "json")]
test_type!(jsonb(
    postgres,
    "jsonb",
    Value::Json(None),
    Value::json(serde_json::json!({"foo": "bar"}))
));

#[cfg(feature = "json")]
test_type!(jsonb_array(
    postgres,
    "jsonb[]",
    Value::Array(None),
    Value::array(vec![
        serde_json::json!({"foo": "bar"}),
        serde_json::json!({"omg": false})
    ])
));

test_type!(xml(postgres, "xml", Value::Xml(None), Value::xml("<test>1</test>",)));

test_type!(xml_array(
    postgres,
    "xml[]",
    Value::Array(None),
    Value::array(vec!["<test>1</test>", "<test>2</test>",])
));

#[cfg(feature = "uuid")]
test_type!(uuid(
    postgres,
    "uuid",
    Value::Uuid(None),
    Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap())
));

#[cfg(feature = "chrono")]
test_type!(date(
    postgres,
    "date",
    Value::Date(None),
    Value::date(chrono::NaiveDate::from_ymd(2020, 4, 20))
));

#[cfg(feature = "chrono")]
test_type!(date_array(
    postgres,
    "date[]",
    Value::Array(None),
    Value::array(vec![chrono::NaiveDate::from_ymd(2020, 4, 20)])
));

#[cfg(feature = "chrono")]
test_type!(time(
    postgres,
    "time",
    Value::Time(None),
    Value::time(chrono::NaiveTime::from_hms(16, 20, 00))
));

#[cfg(feature = "chrono")]
test_type!(time_array(
    postgres,
    "time[]",
    Value::Array(None),
    Value::array(vec![chrono::NaiveTime::from_hms(16, 20, 00)])
));

#[cfg(feature = "chrono")]
test_type!(timestamp(postgres, "timestamp", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(timestamp_array(postgres, "timestamp[]", Value::Array(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::array(vec![dt.with_timezone(&chrono::Utc)])
}));

#[cfg(feature = "chrono")]
test_type!(timestamptz(postgres, "timestamptz", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(timestamptz_array(postgres, "timestamptz[]", Value::Array(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::array(vec![dt.with_timezone(&chrono::Utc)])
}));

test_type!(bytea(
    postgres,
    "bytea",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec())
));

test_type!(bytea_array(
    postgres,
    "bytea[]",
    Value::Array(None),
    Value::array(vec![
        Value::bytes(b"DEADBEEF".to_vec()),
        Value::bytes(b"BEEFBEEF".to_vec())
    ])
));
