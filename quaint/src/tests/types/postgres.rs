#[cfg(feature = "bigdecimal")]
mod bigdecimal;

use crate::tests::test_api::*;
#[cfg(any(feature = "bigdecimal", feature = "uuid"))]
use std::str::FromStr;

test_type!(boolean(
    postgresql,
    "boolean",
    ValueInner::Boolean(None),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(boolean_array(
    postgresql,
    "boolean[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::boolean(true),
        Value::boolean(false),
        Value::boolean(true),
        ValueInner::Boolean(None)
    ]),
));

test_type!(int2(
    postgresql,
    "int2",
    ValueInner::Int32(None),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
));

test_type!(int2_with_int64(
    postgresql,
    "int2",
    (ValueInner::Int64(None), ValueInner::Int32(None)),
    (Value::int64(i16::MIN), Value::int32(i16::MIN)),
    (Value::int64(i16::MAX), Value::int32(i16::MAX))
));

test_type!(int2_array(
    postgresql,
    "int2[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::int32(1),
        Value::int32(2),
        Value::int32(3),
        ValueInner::Int32(None)
    ]),
));

test_type!(int2_array_with_i64(
    postgresql,
    "int2[]",
    (
        Value::array(vec![
            Value::int64(i16::MIN),
            Value::int64(i16::MAX),
            ValueInner::Int64(None)
        ]),
        Value::array(vec![
            Value::int32(i16::MIN),
            Value::int32(i16::MAX),
            ValueInner::Int32(None)
        ])
    )
));

test_type!(int4(
    postgresql,
    "int4",
    ValueInner::Int32(None),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(int4_with_i64(
    postgresql,
    "int4",
    (ValueInner::Int64(None), ValueInner::Int32(None)),
    (Value::int64(i32::MIN), Value::int32(i32::MIN)),
    (Value::int64(i32::MAX), Value::int32(i32::MAX))
));

test_type!(int4_array(
    postgresql,
    "int4[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::int32(i32::MIN),
        Value::int32(i32::MAX),
        ValueInner::Int32(None)
    ]),
));

test_type!(int4_array_with_i64(
    postgresql,
    "int4[]",
    (
        Value::array(vec![
            Value::int64(i32::MIN),
            Value::int64(i32::MAX),
            ValueInner::Int64(None)
        ]),
        Value::array(vec![
            Value::int32(i32::MIN),
            Value::int32(i32::MAX),
            ValueInner::Int32(None)
        ])
    )
));

test_type!(int8(
    postgresql,
    "int8",
    ValueInner::Int64(None),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(int8_array(
    postgresql,
    "int8[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::int64(1),
        Value::int64(2),
        Value::int64(3),
        ValueInner::Int64(None)
    ]),
));

test_type!(float4(
    postgresql,
    "float4",
    ValueInner::Float(None),
    Value::float(1.234)
));

test_type!(float4_array(
    postgresql,
    "float4[]",
    ValueInner::Array(None),
    Value::array(vec![Value::float(1.1234), Value::float(4.321), ValueInner::Float(None)])
));

test_type!(float8(
    postgresql,
    "float8",
    ValueInner::Double(None),
    Value::double(1.12345764),
));

test_type!(float8_array(
    postgresql,
    "float8[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::double(1.1234),
        Value::double(4.321),
        ValueInner::Double(None)
    ])
));

// NOTE: OIDs are unsigned 32-bit integers (see https://www.postgresql.org/docs/9.4/datatype-oid.html)
// but a u32 cannot fit in an i32, so we always read OIDs back from the database as i64s.
test_type!(oid_with_i32(
    postgresql,
    "oid",
    (ValueInner::Int32(None), ValueInner::Int64(None)),
    (Value::int32(i32::MAX), Value::int64(i32::MAX)),
    (Value::int32(u32::MIN as i32), Value::int64(u32::MIN)),
));

test_type!(oid_with_i64(
    postgresql,
    "oid",
    ValueInner::Int64(None),
    Value::int64(u32::MAX),
    Value::int64(u32::MIN),
));

test_type!(oid_array(
    postgresql,
    "oid[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::int64(1),
        Value::int64(2),
        Value::int64(3),
        ValueInner::Int64(None)
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
    ValueInner::Text(None),
    Value::text("foobar")
));

test_type!(char_array(
    postgresql,
    "char(6)[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::text("foobar"),
        Value::text("omgwtf"),
        ValueInner::Text(None)
    ])
));

test_type!(varchar(
    postgresql,
    "varchar(255)",
    ValueInner::Text(None),
    Value::text("foobar")
));

test_type!(varchar_array(
    postgresql,
    "varchar(255)[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::text("foobar"),
        Value::text("omgwtf"),
        ValueInner::Text(None)
    ])
));

test_type!(text(postgresql, "text", ValueInner::Text(None), Value::text("foobar")));

test_type!(text_array(
    postgresql,
    "text[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::text("foobar"),
        Value::text("omgwtf"),
        ValueInner::Text(None)
    ])
));

test_type!(bit(postgresql, "bit(4)", ValueInner::Text(None), Value::text("1001")));

test_type!(bit_array(
    postgresql,
    "bit(4)[]",
    ValueInner::Array(None),
    Value::array(vec![Value::text("1001"), Value::text("0110"), ValueInner::Text(None)])
));

test_type!(varbit(
    postgresql,
    "varbit(20)",
    ValueInner::Text(None),
    Value::text("001010101")
));

test_type!(varbit_array(
    postgresql,
    "varbit(20)[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::text("001010101"),
        Value::text("01101111"),
        ValueInner::Text(None)
    ])
));

test_type!(inet(
    postgresql,
    "inet",
    ValueInner::Text(None),
    Value::text("127.0.0.1")
));

test_type!(inet_array(
    postgresql,
    "inet[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::text("127.0.0.1"),
        Value::text("192.168.1.1"),
        ValueInner::Text(None)
    ])
));

test_type!(json(
    postgresql,
    "json",
    ValueInner::Json(None),
    Value::json(serde_json::json!({"foo": "bar"}))
));

test_type!(json_array(
    postgresql,
    "json[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::json(serde_json::json!({"foo": "bar"})),
        Value::json(serde_json::json!({"omg": false})),
        ValueInner::Json(None)
    ])
));

test_type!(jsonb(
    postgresql,
    "jsonb",
    ValueInner::Json(None),
    Value::json(serde_json::json!({"foo": "bar"}))
));

test_type!(jsonb_array(
    postgresql,
    "jsonb[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::json(serde_json::json!({"foo": "bar"})),
        Value::json(serde_json::json!({"omg": false})),
        ValueInner::Json(None)
    ])
));

test_type!(xml(
    postgresql,
    "xml",
    ValueInner::Xml(None),
    Value::xml("<test>1</test>",)
));

test_type!(xml_array(
    postgresql,
    "xml[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::text("<test>1</test>"),
        Value::text("<test>2</test>"),
        ValueInner::Text(None)
    ])
));

#[cfg(feature = "uuid")]
test_type!(uuid(
    postgresql,
    "uuid",
    ValueInner::Uuid(None),
    Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap())
));

#[cfg(feature = "uuid")]
test_type!(uuid_array(
    postgresql,
    "uuid[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap()),
        ValueInner::Uuid(None)
    ])
));

test_type!(date(
    postgresql,
    "date",
    ValueInner::Date(None),
    Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap())
));

test_type!(date_array(
    postgresql,
    "date[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap()),
        ValueInner::Date(None)
    ])
));

test_type!(time(
    postgresql,
    "time",
    ValueInner::Time(None),
    Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap())
));

test_type!(time_array(
    postgresql,
    "time[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap()),
        ValueInner::Time(None)
    ])
));

test_type!(timestamp(postgresql, "timestamp", ValueInner::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(timestamp_array(postgresql, "timestamp[]", ValueInner::Array(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();

    Value::array(vec![
        Value::datetime(dt.with_timezone(&chrono::Utc)),
        ValueInner::DateTime(None),
    ])
}));

test_type!(timestamptz(postgresql, "timestamptz", ValueInner::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(timestamptz_array(
    postgresql,
    "timestamptz[]",
    ValueInner::Array(None),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();

        Value::array(vec![
            Value::datetime(dt.with_timezone(&chrono::Utc)),
            ValueInner::DateTime(None),
        ])
    }
));

test_type!(bytea(
    postgresql,
    "bytea",
    ValueInner::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec())
));

test_type!(bytea_array(
    postgresql,
    "bytea[]",
    ValueInner::Array(None),
    Value::array(vec![
        Value::bytes(b"DEADBEEF".to_vec()),
        Value::bytes(b"BEEFBEEF".to_vec()),
        ValueInner::Bytes(None)
    ])
));
