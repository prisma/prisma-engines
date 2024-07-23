mod bigdecimal;

use crate::macros::assert_matching_value_and_column_type;
use crate::{connector::ColumnType, tests::test_api::*};
use std::str::FromStr;

test_type!(boolean(
    postgresql,
    "boolean",
    ColumnType::Boolean,
    Value::null_boolean(),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(boolean_array(
    postgresql,
    "boolean[]",
    ColumnType::BooleanArray,
    Value::null_array(),
    Value::array(vec![
        Value::boolean(true),
        Value::boolean(false),
        Value::boolean(true),
        Value::null_boolean()
    ]),
));

test_type!(int2(
    postgresql,
    "int2",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
));

test_type!(int2_with_int64(
    postgresql,
    "int2",
    ColumnType::Int32,
    (Value::null_int64(), Value::null_int32()),
    (Value::int64(i16::MIN), Value::int32(i16::MIN)),
    (Value::int64(i16::MAX), Value::int32(i16::MAX))
));

test_type!(int2_array(
    postgresql,
    "int2[]",
    ColumnType::Int32Array,
    Value::null_array(),
    Value::array(vec![
        Value::int32(1),
        Value::int32(2),
        Value::int32(3),
        Value::null_int32()
    ]),
));

test_type!(int2_array_with_i64(
    postgresql,
    "int2[]",
    ColumnType::Int32Array,
    (
        Value::array(vec![
            Value::int64(i16::MIN),
            Value::int64(i16::MAX),
            Value::null_int64()
        ]),
        Value::array(vec![
            Value::int32(i16::MIN),
            Value::int32(i16::MAX),
            Value::null_int32()
        ])
    )
));

test_type!(int4(
    postgresql,
    "int4",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(int4_with_i64(
    postgresql,
    "int4",
    ColumnType::Int32,
    (Value::null_int64(), Value::null_int32()),
    (Value::int64(i32::MIN), Value::int32(i32::MIN)),
    (Value::int64(i32::MAX), Value::int32(i32::MAX))
));

test_type!(int4_array(
    postgresql,
    "int4[]",
    ColumnType::Int32Array,
    Value::null_array(),
    Value::array(vec![
        Value::int32(i32::MIN),
        Value::int32(i32::MAX),
        Value::null_int32()
    ]),
));

test_type!(int4_array_with_i64(
    postgresql,
    "int4[]",
    ColumnType::Int32Array,
    (
        Value::array(vec![
            Value::int64(i32::MIN),
            Value::int64(i32::MAX),
            Value::null_int64()
        ]),
        Value::array(vec![
            Value::int32(i32::MIN),
            Value::int32(i32::MAX),
            Value::null_int32()
        ])
    )
));

test_type!(int8(
    postgresql,
    "int8",
    ColumnType::Int64,
    Value::null_int64(),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(int8_array(
    postgresql,
    "int8[]",
    ColumnType::Int64Array,
    Value::null_array(),
    Value::array(vec![
        Value::int64(1),
        Value::int64(2),
        Value::int64(3),
        Value::null_int64()
    ]),
));

test_type!(float4(
    postgresql,
    "float4",
    ColumnType::Float,
    Value::null_float(),
    Value::float(1.234)
));

test_type!(float4_array(
    postgresql,
    "float4[]",
    ColumnType::FloatArray,
    Value::null_array(),
    Value::array(vec![Value::float(1.1234), Value::float(4.321), Value::null_float()])
));

test_type!(float8(
    postgresql,
    "float8",
    ColumnType::Double,
    Value::null_double(),
    Value::double(1.12345764),
));

test_type!(float8_array(
    postgresql,
    "float8[]",
    ColumnType::DoubleArray,
    Value::null_array(),
    Value::array(vec![Value::double(1.1234), Value::double(4.321), Value::null_double()])
));

// NOTE: OIDs are unsigned 32-bit integers (see https://www.postgresql.org/docs/9.4/datatype-oid.html)
// but a u32 cannot fit in an i32, so we always read OIDs back from the database as i64s.
test_type!(oid_with_i32(
    postgresql,
    "oid",
    ColumnType::Int64,
    (Value::null_int32(), Value::null_int64()),
    (Value::int32(i32::MAX), Value::int64(i32::MAX)),
    (Value::int32(u32::MIN as i32), Value::int64(u32::MIN)),
));

test_type!(oid_with_i64(
    postgresql,
    "oid",
    ColumnType::Int64,
    Value::null_int64(),
    Value::int64(u32::MAX),
    Value::int64(u32::MIN),
));

test_type!(oid_array(
    postgresql,
    "oid[]",
    ColumnType::Int64Array,
    Value::null_array(),
    Value::array(vec![
        Value::int64(1),
        Value::int64(2),
        Value::int64(3),
        Value::null_int64()
    ]),
));

test_type!(serial2(
    postgresql,
    "serial2",
    ColumnType::Int32,
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
));

test_type!(serial4(
    postgresql,
    "serial4",
    ColumnType::Int32,
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(serial8(
    postgresql,
    "serial8",
    ColumnType::Int64,
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(char(
    postgresql,
    "char(6)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));

test_type!(char_array(
    postgresql,
    "char(6)[]",
    ColumnType::TextArray,
    Value::null_array(),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf"), Value::null_text()])
));

test_type!(varchar(
    postgresql,
    "varchar(255)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));

test_type!(varchar_array(
    postgresql,
    "varchar(255)[]",
    ColumnType::TextArray,
    Value::null_array(),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf"), Value::null_text()])
));

test_type!(text(
    postgresql,
    "text",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));

test_type!(text_array(
    postgresql,
    "text[]",
    ColumnType::TextArray,
    Value::null_array(),
    Value::array(vec![Value::text("foobar"), Value::text("omgwtf"), Value::null_text()])
));

test_type!(bit(
    postgresql,
    "bit(4)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("1001")
));

test_type!(bit_array(
    postgresql,
    "bit(4)[]",
    ColumnType::TextArray,
    Value::null_array(),
    Value::array(vec![Value::text("1001"), Value::text("0110"), Value::null_text()])
));

test_type!(varbit(
    postgresql,
    "varbit(20)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("001010101")
));

test_type!(varbit_array(
    postgresql,
    "varbit(20)[]",
    ColumnType::TextArray,
    Value::null_array(),
    Value::array(vec![
        Value::text("001010101"),
        Value::text("01101111"),
        Value::null_text()
    ])
));

test_type!(inet(
    postgresql,
    "inet",
    ColumnType::Text,
    Value::null_text(),
    Value::text("127.0.0.1")
));

test_type!(inet_array(
    postgresql,
    "inet[]",
    ColumnType::TextArray,
    Value::null_array(),
    Value::array(vec![
        Value::text("127.0.0.1"),
        Value::text("192.168.1.1"),
        Value::null_text()
    ])
));

test_type!(json(
    postgresql,
    "json",
    ColumnType::Json,
    Value::null_json(),
    Value::json(serde_json::json!({"foo": "bar"}))
));

test_type!(json_array(
    postgresql,
    "json[]",
    ColumnType::JsonArray,
    Value::null_array(),
    Value::array(vec![
        Value::json(serde_json::json!({"foo": "bar"})),
        Value::json(serde_json::json!({"omg": false})),
        Value::null_json()
    ])
));

test_type!(jsonb(
    postgresql,
    "jsonb",
    ColumnType::Json,
    Value::null_json(),
    Value::json(serde_json::json!({"foo": "bar"}))
));

test_type!(jsonb_array(
    postgresql,
    "jsonb[]",
    ColumnType::JsonArray,
    Value::null_array(),
    Value::array(vec![
        Value::json(serde_json::json!({"foo": "bar"})),
        Value::json(serde_json::json!({"omg": false})),
        Value::null_json()
    ])
));

test_type!(xml(
    postgresql,
    "xml",
    ColumnType::Xml,
    Value::null_xml(),
    Value::xml("<test>1</test>",)
));

test_type!(xml_array(
    postgresql,
    "xml[]",
    ColumnType::TextArray,
    Value::null_array(),
    Value::array(vec![
        Value::text("<test>1</test>"),
        Value::text("<test>2</test>"),
        Value::null_text()
    ])
));

test_type!(uuid(
    postgresql,
    "uuid",
    ColumnType::Uuid,
    Value::null_uuid(),
    Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap())
));

test_type!(uuid_array(
    postgresql,
    "uuid[]",
    ColumnType::UuidArray,
    Value::null_array(),
    Value::array(vec![
        Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap()),
        Value::null_uuid(),
    ])
));

test_type!(date(
    postgresql,
    "date",
    ColumnType::Date,
    Value::null_date(),
    Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap())
));

test_type!(date_array(
    postgresql,
    "date[]",
    ColumnType::DateArray,
    Value::null_array(),
    Value::array(vec![
        Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap()),
        Value::null_date()
    ])
));

test_type!(time(
    postgresql,
    "time",
    ColumnType::Time,
    Value::null_time(),
    Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap())
));

test_type!(time_array(
    postgresql,
    "time[]",
    ColumnType::TimeArray,
    Value::null_array(),
    Value::array(vec![
        Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap()),
        Value::null_time()
    ])
));

test_type!(timestamp(
    postgresql,
    "timestamp",
    ColumnType::DateTime,
    Value::null_datetime(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
        Value::datetime(dt.with_timezone(&chrono::Utc))
    }
));

test_type!(timestamp_array(
    postgresql,
    "timestamp[]",
    ColumnType::DateTimeArray,
    Value::null_array(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();

        Value::array(vec![
            Value::datetime(dt.with_timezone(&chrono::Utc)),
            Value::null_datetime(),
        ])
    }
));

test_type!(timestamptz(
    postgresql,
    "timestamptz",
    ColumnType::DateTime,
    Value::null_datetime(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
        Value::datetime(dt.with_timezone(&chrono::Utc))
    }
));

test_type!(timestamptz_array(
    postgresql,
    "timestamptz[]",
    ColumnType::DateTimeArray,
    Value::null_array(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();

        Value::array(vec![
            Value::datetime(dt.with_timezone(&chrono::Utc)),
            Value::null_datetime(),
        ])
    }
));

test_type!(bytea(
    postgresql,
    "bytea",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(b"DEADBEEF".to_vec())
));

test_type!(bytea_array(
    postgresql,
    "bytea[]",
    ColumnType::BytesArray,
    Value::null_array(),
    Value::array(vec![
        Value::bytes(b"DEADBEEF".to_vec()),
        Value::bytes(b"BEEFBEEF".to_vec()),
        Value::null_bytes()
    ])
));
