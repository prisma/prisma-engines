#[cfg(feature = "bigdecimal")]
mod bigdecimal;

use crate::tests::test_api::*;
#[cfg(feature = "bigdecimal")]
use std::str::FromStr;

test_type!(boolean(
    postgresql,
    "boolean",
    Value::Boolean(None),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(boolean_array(
    postgresql,
    "boolean[]",
    Value::Array(None),
    Value::array(vec![true, false, true]),
));

test_type!(int2(
    postgresql,
    "int2",
    Value::Integer(None),
    Value::integer(i16::MIN),
    Value::integer(i16::MAX),
));

test_type!(int2_array(
    postgresql,
    "int2[]",
    Value::Array(None),
    Value::array(vec![1, 2, 3]),
));

test_type!(int4(
    postgresql,
    "int4",
    Value::Integer(None),
    Value::integer(i32::MIN),
    Value::integer(i32::MAX),
));

test_type!(int4_array(
    postgresql,
    "int4[]",
    Value::Array(None),
    Value::array(vec![1, 2, 3]),
));

test_type!(int8(
    postgresql,
    "int8",
    Value::Integer(None),
    Value::integer(i64::MIN),
    Value::integer(i64::MAX),
));

test_type!(int8_array(
    postgresql,
    "int8[]",
    Value::Array(None),
    Value::array(vec![1, 2, 3]),
));

test_type!(float4(postgresql, "float4", Value::Float(None), Value::float(1.234)));

test_type!(float4_array(
    postgresql,
    "float4[]",
    Value::Array(None),
    Value::array(vec![1.1234_f32, 4.321_f32])
));

test_type!(float8(
    postgresql,
    "float8",
    Value::Double(None),
    Value::double(1.12345764),
));

test_type!(float8_array(
    postgresql,
    "float8[]",
    Value::Array(None),
    Value::array(vec![1.1234_f64, 4.321_f64])
));

test_type!(oid(postgresql, "oid", Value::Integer(None), Value::integer(10000)));

test_type!(oid_array(
    postgresql,
    "oid[]",
    Value::Array(None),
    Value::array(vec![1, 2, 3]),
));

test_type!(serial2(
    postgresql,
    "serial2",
    Value::integer(i16::MIN),
    Value::integer(i16::MAX),
));

test_type!(serial4(
    postgresql,
    "serial4",
    Value::integer(i32::MIN),
    Value::integer(i32::MAX),
));

test_type!(serial8(
    postgresql,
    "serial8",
    Value::integer(i64::MIN),
    Value::integer(i64::MAX),
));

test_type!(char(postgresql, "char(6)", Value::Text(None), Value::text("foobar")));

test_type!(char_array(
    postgresql,
    "char(6)[]",
    Value::Array(None),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf")])
));

test_type!(varchar(
    postgresql,
    "varchar(255)",
    Value::Text(None),
    Value::text("foobar")
));

test_type!(varchar_array(
    postgresql,
    "varchar(255)[]",
    Value::Array(None),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf")])
));

test_type!(text(postgresql, "text", Value::Text(None), Value::text("foobar")));

test_type!(text_array(
    postgresql,
    "text[]",
    Value::Array(None),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf")])
));

test_type!(bit(postgresql, "bit(4)", Value::Text(None), Value::text("1001")));

test_type!(bit_array(
    postgresql,
    "bit(4)[]",
    Value::Array(None),
    Value::array(vec![Value::text("1001"), Value::text("0110")])
));

test_type!(varbit(
    postgresql,
    "varbit(20)",
    Value::Text(None),
    Value::text("001010101")
));

test_type!(varbit_array(
    postgresql,
    "varbit(20)[]",
    Value::Array(None),
    Value::array(vec![Value::text("001010101"), Value::text("01101111")])
));

test_type!(inet(postgresql, "inet", Value::Text(None), Value::text("127.0.0.1")));

test_type!(inet_array(
    postgresql,
    "inet[]",
    Value::Array(None),
    Value::array(vec![Value::text("127.0.0.1"), Value::text("192.168.1.1")])
));

#[cfg(feature = "json")]
test_type!(json(
    postgresql,
    "json",
    Value::Json(None),
    Value::json(serde_json::json!({"foo": "bar"}))
));

#[cfg(feature = "json")]
test_type!(json_array(
    postgresql,
    "json[]",
    Value::Array(None),
    Value::array(vec![
        serde_json::json!({"foo": "bar"}),
        serde_json::json!({"omg": false})
    ])
));

#[cfg(feature = "json")]
test_type!(jsonb(
    postgresql,
    "jsonb",
    Value::Json(None),
    Value::json(serde_json::json!({"foo": "bar"}))
));

#[cfg(feature = "json")]
test_type!(jsonb_array(
    postgresql,
    "jsonb[]",
    Value::Array(None),
    Value::array(vec![
        serde_json::json!({"foo": "bar"}),
        serde_json::json!({"omg": false})
    ])
));

test_type!(xml(postgresql, "xml", Value::Xml(None), Value::xml("<test>1</test>",)));

test_type!(xml_array(
    postgresql,
    "xml[]",
    Value::Array(None),
    Value::array(vec!["<test>1</test>", "<test>2</test>",])
));

#[cfg(feature = "uuid")]
test_type!(uuid(
    postgresql,
    "uuid",
    Value::Uuid(None),
    Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap())
));

#[cfg(feature = "chrono")]
test_type!(date(
    postgresql,
    "date",
    Value::Date(None),
    Value::date(chrono::NaiveDate::from_ymd(2020, 4, 20))
));

#[cfg(feature = "chrono")]
test_type!(date_array(
    postgresql,
    "date[]",
    Value::Array(None),
    Value::array(vec![chrono::NaiveDate::from_ymd(2020, 4, 20)])
));

#[cfg(feature = "chrono")]
test_type!(time(
    postgresql,
    "time",
    Value::Time(None),
    Value::time(chrono::NaiveTime::from_hms(16, 20, 00))
));

#[cfg(feature = "chrono")]
test_type!(time_array(
    postgresql,
    "time[]",
    Value::Array(None),
    Value::array(vec![chrono::NaiveTime::from_hms(16, 20, 00)])
));

#[cfg(feature = "chrono")]
test_type!(timestamp(postgresql, "timestamp", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(timestamp_array(postgresql, "timestamp[]", Value::Array(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::array(vec![dt.with_timezone(&chrono::Utc)])
}));

#[cfg(feature = "chrono")]
test_type!(timestamptz(postgresql, "timestamptz", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(timestamptz_array(postgresql, "timestamptz[]", Value::Array(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::array(vec![dt.with_timezone(&chrono::Utc)])
}));

test_type!(bytea(
    postgresql,
    "bytea",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec())
));

test_type!(bytea_array(
    postgresql,
    "bytea[]",
    Value::Array(None),
    Value::array(vec![
        Value::bytes(b"DEADBEEF".to_vec()),
        Value::bytes(b"BEEFBEEF".to_vec())
    ])
));
