use crate::tests::test_api::*;
use rust_decimal::Decimal;
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

test_type!(decimal(
    postgres,
    "decimal(10,2)",
    Value::Real(None),
    Value::real(Decimal::new(314, 2))
));

test_type!(decimal_10_2(
    postgres,
    "decimal(10, 2)",
    (
        Value::real(Decimal::from_str("3950.123456")?),
        Value::real(Decimal::from_str("3950.12")?)
    )
));

test_type!(decimal_35_6(
    postgres,
    "decimal(35, 6)",
    (
        Value::real(Decimal::from_str("3950")?),
        Value::real(Decimal::from_str("3950.000000")?)
    ),
    (
        Value::real(Decimal::from_str("3950.123456")?),
        Value::real(Decimal::from_str("3950.123456")?)
    ),
    (
        Value::real(Decimal::from_str("0.1")?),
        Value::real(Decimal::from_str("0.100000")?)
    ),
    (
        Value::real(Decimal::from_str("0.01")?),
        Value::real(Decimal::from_str("0.010000")?)
    ),
    (
        Value::real(Decimal::from_str("0.001")?),
        Value::real(Decimal::from_str("0.001000")?)
    ),
    (
        Value::real(Decimal::from_str("0.0001")?),
        Value::real(Decimal::from_str("0.000100")?)
    ),
    (
        Value::real(Decimal::from_str("0.00001")?),
        Value::real(Decimal::from_str("0.000010")?)
    ),
    (
        Value::real(Decimal::from_str("0.000001")?),
        Value::real(Decimal::from_str("0.000001")?)
    ),
    (
        Value::real(Decimal::from_str("1")?),
        Value::real(Decimal::from_str("1.000000")?)
    ),
    (
        Value::real(Decimal::from_str("-100")?),
        Value::real(Decimal::from_str("-100.000000")?)
    ),
    (
        Value::real(Decimal::from_str("-123.456")?),
        Value::real(Decimal::from_str("-123.456000")?)
    ),
    (
        Value::real(Decimal::from_str("119996.25")?),
        Value::real(Decimal::from_str("119996.250000")?)
    ),
    (
        Value::real(Decimal::from_str("1000000")?),
        Value::real(Decimal::from_str("1000000.000000")?)
    ),
    (
        Value::real(Decimal::from_str("9999999.99999")?),
        Value::real(Decimal::from_str("9999999.999990")?)
    ),
    (
        Value::real(Decimal::from_str("12340.56789")?),
        Value::real(Decimal::from_str("12340.567890")?)
    ),
    (
        Value::real(Decimal::from_str("18446744073709551615")?),
        Value::real(Decimal::from_str("18446744073709551615.000000")?)
    ),
    (
        Value::real(Decimal::from_str("-18446744073709551615")?),
        Value::real(Decimal::from_str("-18446744073709551615.000000")?)
    ),
    (
        Value::real(Decimal::from_str("0.10001")?),
        Value::real(Decimal::from_str("0.100010")?)
    ),
    (
        Value::real(Decimal::from_str("0.12345")?),
        Value::real(Decimal::from_str("0.123450")?)
    ),
));

test_type!(decimal_35_2(
    postgres,
    "decimal(35, 2)",
    (
        Value::real(Decimal::from_str("3950.123456")?),
        Value::real(Decimal::from_str("3950.12")?)
    ),
    (
        Value::real(Decimal::from_str("3950.1256")?),
        Value::real(Decimal::from_str("3950.13")?)
    ),
));

test_type!(decimal_4_0(
    postgres,
    "decimal(4, 0)",
    Value::real(Decimal::from_str("3950")?)
));

test_type!(decimal_65_30(
    postgres,
    "decimal(65, 30)",
    (
        Value::real(Decimal::from_str("1.2")?),
        Value::real(Decimal::from_str("1.2000000000000000000000000000")?)
    ),
    (
        Value::real(Decimal::from_str("3.141592653589793238462643383279")?),
        Value::real(Decimal::from_str("3.1415926535897932384626433833")?)
    )
));

test_type!(decimal_65_34(
    postgres,
    "decimal(65, 34)",
    (
        Value::real(Decimal::from_str("3.1415926535897932384626433832795028")?),
        Value::real(Decimal::from_str("3.1415926535897932384626433833")?)
    ),
    (
        Value::real(Decimal::from_str("1.234567890123456789012345678950000")?),
        Value::real(Decimal::from_str("1.2345678901234567890123456790")?)
    ),
    (
        Value::real(Decimal::from_str("1.234567890123456789012345678949999")?),
        Value::real(Decimal::from_str("1.2345678901234567890123456789")?)
    ),
));

test_type!(decimal_35_0(
    postgres,
    "decimal(35, 0)",
    Value::real(Decimal::from_str("79228162514264337593543950335")?),
));

test_type!(decimal_35_1(
    postgres,
    "decimal(35, 1)",
    (
        Value::real(Decimal::from_str("79228162514264337593543950335")?),
        Value::real(Decimal::from_str("79228162514264337593543950335.0")?)
    ),
    (
        Value::real(Decimal::from_str("4951760157141521099596496896")?),
        Value::real(Decimal::from_str("4951760157141521099596496896.0")?)
    )
));

test_type!(decimal_array(
    postgres,
    "decimal(10,2)[]",
    Value::Array(None),
    Value::array(vec![Decimal::new(314, 2), Decimal::new(512, 2)])
));

test_type!(float4(
    postgres,
    "float4",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::from_str("1.1234").unwrap())
));

test_type!(float4_array(
    postgres,
    "float4[]",
    Value::Array(None),
    Value::array(vec![
        rust_decimal::Decimal::from_str("1.1234").unwrap(),
        rust_decimal::Decimal::from_str("4.3210").unwrap(),
    ])
));

test_type!(float8(
    postgres,
    "float8",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::from_str("1.12345").unwrap())
));

test_type!(float8_array(
    postgres,
    "float8[]",
    Value::Array(None),
    Value::array(vec![
        rust_decimal::Decimal::from_str("1.1234").unwrap(),
        rust_decimal::Decimal::from_str("4.3210").unwrap(),
    ])
));

test_type!(money(
    postgres,
    "money",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::from_str("1.12").unwrap())
));

test_type!(money_array(
    postgres,
    "money[]",
    Value::Array(None),
    Value::array(vec![
        rust_decimal::Decimal::from_str("1.12").unwrap(),
        rust_decimal::Decimal::from_str("1.12").unwrap()
    ])
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

#[cfg(feature = "json-1")]
test_type!(json(
    postgres,
    "json",
    Value::Json(None),
    Value::json(serde_json::json!({"foo": "bar"}))
));

#[cfg(feature = "json-1")]
test_type!(json_array(
    postgres,
    "json[]",
    Value::Array(None),
    Value::array(vec![
        serde_json::json!({"foo": "bar"}),
        serde_json::json!({"omg": false})
    ])
));

#[cfg(feature = "json-1")]
test_type!(jsonb(
    postgres,
    "jsonb",
    Value::Json(None),
    Value::json(serde_json::json!({"foo": "bar"}))
));

#[cfg(feature = "json-1")]
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

#[cfg(feature = "uuid-0_8")]
test_type!(uuid(
    postgres,
    "uuid",
    Value::Uuid(None),
    Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap())
));

#[cfg(feature = "chrono-0_4")]
test_type!(date(
    postgres,
    "date",
    Value::Date(None),
    Value::date(chrono::NaiveDate::from_ymd(2020, 4, 20))
));

#[cfg(feature = "chrono-0_4")]
test_type!(date_array(
    postgres,
    "date[]",
    Value::Array(None),
    Value::array(vec![chrono::NaiveDate::from_ymd(2020, 4, 20)])
));

#[cfg(feature = "chrono-0_4")]
test_type!(time(
    postgres,
    "time",
    Value::Time(None),
    Value::time(chrono::NaiveTime::from_hms(16, 20, 00))
));

#[cfg(feature = "chrono-0_4")]
test_type!(time_array(
    postgres,
    "time[]",
    Value::Array(None),
    Value::array(vec![chrono::NaiveTime::from_hms(16, 20, 00)])
));

#[cfg(feature = "chrono-0_4")]
test_type!(timestamp(postgres, "timestamp", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono-0_4")]
test_type!(timestamp_array(postgres, "timestamp[]", Value::Array(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::array(vec![dt.with_timezone(&chrono::Utc)])
}));

#[cfg(feature = "chrono-0_4")]
test_type!(timestamptz(postgres, "timestamptz", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono-0_4")]
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
