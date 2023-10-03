#[cfg(feature = "bigdecimal")]
mod bigdecimal;

use crate::tests::test_api::*;
#[cfg(any(feature = "bigdecimal", feature = "uuid"))]
use std::str::FromStr;

test_type!(boolean(
    postgresql,
    "boolean",
    ValueType::Boolean(None).into_value(),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(boolean_array(
    postgresql,
    "boolean[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::boolean(true),
        Value::boolean(false),
        Value::boolean(true),
        ValueType::Boolean(None).into_value()
    ]),
));

test_type!(int2(
    postgresql,
    "int2",
    ValueType::Int32(None).into_value(),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
));

test_type!(int2_with_int64(
    postgresql,
    "int2",
    (ValueType::Int64(None).into_value(), ValueType::Int32(None).into_value()),
    (Value::int64(i16::MIN), Value::int32(i16::MIN)),
    (Value::int64(i16::MAX), Value::int32(i16::MAX))
));

test_type!(int2_array(
    postgresql,
    "int2[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::int32(1),
        Value::int32(2),
        Value::int32(3),
        ValueType::Int32(None).into_value()
    ]),
));

test_type!(int2_array_with_i64(
    postgresql,
    "int2[]",
    (
        Value::array(vec![
            Value::int64(i16::MIN),
            Value::int64(i16::MAX),
            ValueType::Int64(None).into_value()
        ]),
        Value::array(vec![
            Value::int32(i16::MIN),
            Value::int32(i16::MAX),
            ValueType::Int32(None).into_value()
        ])
    )
));

test_type!(int4(
    postgresql,
    "int4",
    ValueType::Int32(None).into_value(),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(int4_with_i64(
    postgresql,
    "int4",
    (ValueType::Int64(None).into_value(), ValueType::Int32(None).into_value()),
    (Value::int64(i32::MIN), Value::int32(i32::MIN)),
    (Value::int64(i32::MAX), Value::int32(i32::MAX))
));

test_type!(int4_array(
    postgresql,
    "int4[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::int32(i32::MIN),
        Value::int32(i32::MAX),
        ValueType::Int32(None).into_value()
    ]),
));

test_type!(int4_array_with_i64(
    postgresql,
    "int4[]",
    (
        Value::array(vec![
            Value::int64(i32::MIN),
            Value::int64(i32::MAX),
            ValueType::Int64(None).into_value()
        ]),
        Value::array(vec![
            Value::int32(i32::MIN),
            Value::int32(i32::MAX),
            ValueType::Int32(None).into_value()
        ])
    )
));

test_type!(int8(
    postgresql,
    "int8",
    ValueType::Int64(None).into_value(),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(int8_array(
    postgresql,
    "int8[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::int64(1),
        Value::int64(2),
        Value::int64(3),
        ValueType::Int64(None).into_value()
    ]),
));

test_type!(float4(
    postgresql,
    "float4",
    ValueType::Float(None).into_value(),
    Value::float(1.234)
));

test_type!(float4_array(
    postgresql,
    "float4[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![Value::float(1.1234), Value::float(4.321), ValueType::Float(None).into_value()])
));

test_type!(float8(
    postgresql,
    "float8",
    ValueType::Double(None).into_value(),
    Value::double(1.12345764),
));

test_type!(float8_array(
    postgresql,
    "float8[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::double(1.1234),
        Value::double(4.321),
        ValueType::Double(None).into_value()
    ])
));

// NOTE: OIDs are unsigned 32-bit integers (see https://www.postgresql.org/docs/9.4/datatype-oid.html)
// but a u32 cannot fit in an i32, so we always read OIDs back from the database as i64s.
test_type!(oid_with_i32(
    postgresql,
    "oid",
    (ValueType::Int32(None).into_value(), ValueType::Int64(None).into_value()),
    (Value::int32(i32::MAX), Value::int64(i32::MAX)),
    (Value::int32(u32::MIN as i32), Value::int64(u32::MIN)),
));

test_type!(oid_with_i64(
    postgresql,
    "oid",
    ValueType::Int64(None).into_value(),
    Value::int64(u32::MAX),
    Value::int64(u32::MIN),
));

test_type!(oid_array(
    postgresql,
    "oid[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::int64(1),
        Value::int64(2),
        Value::int64(3),
        ValueType::Int64(None).into_value()
    ]),
));

test_type!(serial2(
    postgresql,
    "serial2",
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
));

test_type!(serial4(
    postgresql,
    "serial4",
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(serial8(
    postgresql,
    "serial8",
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(char(
    postgresql,
    "char(6)",
    ValueType::Text(None).into_value(),
    Value::text("foobar")
));

test_type!(char_array(
    postgresql,
    "char(6)[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::text("foobar"),
        Value::text("omgwtf"),
        ValueType::Text(None).into_value()
    ])
));

test_type!(varchar(
    postgresql,
    "varchar(255)",
    ValueType::Text(None).into_value(),
    Value::text("foobar")
));

test_type!(varchar_array(
    postgresql,
    "varchar(255)[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::text("foobar"),
        Value::text("omgwtf"),
        ValueType::Text(None).into_value()
    ])
));

test_type!(text(postgresql, "text", ValueType::Text(None).into_value(), Value::text("foobar")));

test_type!(text_array(
    postgresql,
    "text[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::text("foobar"),
        Value::text("omgwtf"),
        ValueType::Text(None).into_value()
    ])
));

test_type!(bit(postgresql, "bit(4)", ValueType::Text(None).into_value(), Value::text("1001")));

test_type!(bit_array(
    postgresql,
    "bit(4)[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![Value::text("1001"), Value::text("0110"), ValueType::Text(None).into_value()])
));

test_type!(varbit(
    postgresql,
    "varbit(20)",
    ValueType::Text(None).into_value(),
    Value::text("001010101")
));

test_type!(varbit_array(
    postgresql,
    "varbit(20)[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::text("001010101"),
        Value::text("01101111"),
        ValueType::Text(None).into_value()
    ])
));

test_type!(inet(
    postgresql,
    "inet",
    ValueType::Text(None).into_value(),
    Value::text("127.0.0.1")
));

test_type!(inet_array(
    postgresql,
    "inet[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::text("127.0.0.1"),
        Value::text("192.168.1.1"),
        ValueType::Text(None).into_value()
    ])
));

test_type!(json(
    postgresql,
    "json",
    ValueType::Json(None).into_value(),
    Value::json(serde_json::json!({"foo": "bar"}))
));

test_type!(json_array(
    postgresql,
    "json[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::json(serde_json::json!({"foo": "bar"})),
        Value::json(serde_json::json!({"omg": false})),
        ValueType::Json(None).into_value()
    ])
));

test_type!(jsonb(
    postgresql,
    "jsonb",
    ValueType::Json(None).into_value(),
    Value::json(serde_json::json!({"foo": "bar"}))
));

test_type!(jsonb_array(
    postgresql,
    "jsonb[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::json(serde_json::json!({"foo": "bar"})),
        Value::json(serde_json::json!({"omg": false})),
        ValueType::Json(None).into_value()
    ])
));

test_type!(xml(
    postgresql,
    "xml",
    ValueType::Xml(None).into_value(),
    Value::xml("<test>1</test>",)
));

test_type!(xml_array(
    postgresql,
    "xml[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::text("<test>1</test>"),
        Value::text("<test>2</test>"),
        ValueType::Text(None).into_value()
    ])
));

#[cfg(feature = "uuid")]
test_type!(uuid(
    postgresql,
    "uuid",
    ValueType::Uuid(None).into_value(),
    Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap())
));

#[cfg(feature = "uuid")]
test_type!(uuid_array(
    postgresql,
    "uuid[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap()),
        ValueType::Uuid(None).into_value()
    ])
));

test_type!(date(
    postgresql,
    "date",
    ValueType::Date(None).into_value(),
    Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap())
));

test_type!(date_array(
    postgresql,
    "date[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap()),
        ValueType::Date(None).into_value()
    ])
));

test_type!(time(
    postgresql,
    "time",
    ValueType::Time(None).into_value(),
    Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap())
));

test_type!(time_array(
    postgresql,
    "time[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap()),
        ValueType::Time(None).into_value()
    ])
));

test_type!(timestamp(postgresql, "timestamp", ValueType::DateTime(None).into_value(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(timestamp_array(postgresql, "timestamp[]", ValueType::Array(None).into_value(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();

    Value::array(vec![
        Value::datetime(dt.with_timezone(&chrono::Utc)),
        ValueType::DateTime(None).into_value(),
    ])
}));

test_type!(timestamptz(postgresql, "timestamptz", ValueType::DateTime(None).into_value(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(timestamptz_array(
    postgresql,
    "timestamptz[]",
    ValueType::Array(None).into_value(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();

        Value::array(vec![
            Value::datetime(dt.with_timezone(&chrono::Utc)),
            ValueType::DateTime(None).into_value(),
        ])
    }
));

test_type!(bytea(
    postgresql,
    "bytea",
    ValueType::Bytes(None).into_value(),
    Value::bytes(b"DEADBEEF".to_vec())
));

test_type!(bytea_array(
    postgresql,
    "bytea[]",
    ValueType::Array(None).into_value(),
    Value::array(vec![
        Value::bytes(b"DEADBEEF".to_vec()),
        Value::bytes(b"BEEFBEEF".to_vec()),
        ValueType::Bytes(None).into_value()
    ])
));
