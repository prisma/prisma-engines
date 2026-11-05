//! Real-connection type coverage for KingbaseES Oracle-compatible mode.
//!
//! These tests deliberately do not reuse `types/mysql.rs` or
//! `types/postgres.rs`. Oracle-mode types use different DDL names and
//! PostgreSQL-wire OIDs, and arrays are intentionally unsupported.

use crate::{connector::ColumnType, macros::assert_matching_value_and_column_type, tests::test_api::*};
use std::str::FromStr;

use crate::bigdecimal::BigDecimal;

test_type!(number_integer(
    kingbase_oracle,
    "NUMBER(10,0)",
    ColumnType::Numeric,
    (Value::null_int32(), Value::null_numeric()),
    (Value::int32(-42), Value::numeric(BigDecimal::from_str("-42").unwrap())),
    (Value::int32(42), Value::numeric(BigDecimal::from_str("42").unwrap()))
));

test_type!(number_bigint(
    kingbase_oracle,
    "NUMBER(19,0)",
    ColumnType::Numeric,
    (
        Value::int64(i64::MIN),
        Value::numeric(BigDecimal::from_str("-9223372036854775808").unwrap())
    ),
    (
        Value::int64(i64::MAX),
        Value::numeric(BigDecimal::from_str("9223372036854775807").unwrap())
    )
));

test_type!(number_with_precision(
    kingbase_oracle,
    "NUMBER(19)",
    ColumnType::Numeric,
    (
        Value::int64(i64::MAX),
        Value::numeric(BigDecimal::from_str("9223372036854775807").unwrap())
    )
));

test_type!(number_decimal(
    kingbase_oracle,
    "NUMBER(20,6)",
    ColumnType::Numeric,
    Value::null_numeric(),
    Value::numeric(BigDecimal::from_str("12345678901234.123456").unwrap())
));

test_type!(number_without_precision(
    kingbase_oracle,
    "NUMBER",
    ColumnType::Numeric,
    Value::null_numeric(),
    Value::numeric(BigDecimal::from_str("12345678901234567890.123456789").unwrap())
));

test_type!(number_boolean_precision(
    kingbase_oracle,
    "NUMBER(1,0)",
    ColumnType::Numeric,
    (Value::null_boolean(), Value::null_numeric()),
    (
        Value::boolean(false),
        Value::numeric(BigDecimal::from_str("0").unwrap())
    ),
    (Value::boolean(true), Value::numeric(BigDecimal::from_str("1").unwrap()))
));

test_type!(binary_float(
    kingbase_oracle,
    "BINARY_FLOAT",
    ColumnType::Float,
    Value::null_float(),
    Value::float(1.25)
));

test_type!(binary_double(
    kingbase_oracle,
    "BINARY_DOUBLE",
    ColumnType::Double,
    Value::null_double(),
    Value::double(1.234_567_89)
));

test_type!(float_with_precision(
    kingbase_oracle,
    "FLOAT(53)",
    ColumnType::Double,
    Value::null_double(),
    Value::double(1.234_567_89)
));

test_type!(float_without_precision(
    kingbase_oracle,
    "FLOAT",
    ColumnType::Double,
    Value::null_double(),
    Value::double(1.234_567_89)
));

test_type!(float_minimum_precision(
    kingbase_oracle,
    "FLOAT(1)",
    ColumnType::Float,
    Value::null_float(),
    Value::float(1.25)
));

test_type!(varchar2(
    kingbase_oracle,
    "VARCHAR2(40)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("Kingbase 金仓")
));

test_type!(varchar2_four_thousand_characters(
    kingbase_oracle,
    "VARCHAR2(4000)",
    ColumnType::Text,
    Value::text("x".repeat(4000))
));

test_type!(char(
    kingbase_oracle,
    "CHAR(10)",
    ColumnType::Text,
    (Value::null_text(), Value::null_text()),
    // CHAR is fixed-width in Oracle mode, as is NCHAR below.
    (Value::text("oracle"), Value::text("oracle    "))
));

test_type!(char_without_length(
    kingbase_oracle,
    "CHAR",
    ColumnType::Text,
    Value::null_text(),
    Value::text("O")
));

test_type!(varchar2_without_length(
    kingbase_oracle,
    "VARCHAR2",
    ColumnType::Text,
    Value::null_text(),
    Value::text("O")
));

test_type!(nchar(
    kingbase_oracle,
    "NCHAR(10)",
    ColumnType::Text,
    (Value::null_text(), Value::null_text()),
    // NCHAR is fixed-width in Oracle mode, so the server pads this two-character
    // value to the declared length before returning it.
    (Value::text("金仓"), Value::text("金仓        "))
));

test_type!(nchar_without_length(
    kingbase_oracle,
    "NCHAR",
    ColumnType::Text,
    Value::null_text(),
    Value::text("金")
));

test_type!(nvarchar2(
    kingbase_oracle,
    "NVARCHAR2(40)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("金仓 Oracle")
));

test_type!(nvarchar2_without_length(
    kingbase_oracle,
    "NVARCHAR2",
    ColumnType::Text,
    Value::null_text(),
    Value::text("金")
));

test_type!(clob(
    kingbase_oracle,
    "CLOB",
    ColumnType::Text,
    (Value::null_text(), Value::null_text()),
    (
        Value::text("large text").with_native_column_type(Some("Clob")),
        Value::text("large text")
    )
));

test_type!(nclob(
    kingbase_oracle,
    "NCLOB",
    ColumnType::Text,
    (Value::null_text(), Value::null_text()),
    (
        Value::text("国家文本").with_native_column_type(Some("NClob")),
        Value::text("国家文本")
    )
));

test_type!(blob(
    kingbase_oracle,
    "BLOB",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(vec![0, 1, 2, 255])
));

test_type!(boolean(
    kingbase_oracle,
    "BOOLEAN",
    ColumnType::Boolean,
    (Value::null_boolean(), Value::null_boolean()),
    (
        Value::boolean(false).with_native_column_type(Some("Boolean")),
        Value::boolean(false)
    ),
    (
        Value::boolean(true).with_native_column_type(Some("Boolean")),
        Value::boolean(true)
    )
));

test_type!(json(
    kingbase_oracle,
    "JSON",
    ColumnType::Json,
    Value::null_json(),
    Value::json(serde_json::json!({ "mode": "oracle", "items": [1, 2] }))
));

test_type!(xml(
    kingbase_oracle,
    "XML",
    ColumnType::Xml,
    Value::null_xml(),
    Value::xml("<kingbase>oracle</kingbase>")
));

test_type!(uuid(
    kingbase_oracle,
    "UUID",
    ColumnType::Uuid,
    (Value::null_text(), Value::null_uuid()),
    (
        Value::text("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").with_native_column_type(Some("Uuid")),
        Value::uuid(uuid::Uuid::parse_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap())
    )
));

test_type!(date(
    kingbase_oracle,
    "DATE",
    ColumnType::DateTime,
    Value::null_datetime(),
    Value::datetime(
        chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    )
));

test_type!(timestamp(
    kingbase_oracle,
    "TIMESTAMP(6)",
    ColumnType::DateTime,
    Value::null_datetime(),
    Value::datetime(
        chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    )
));

test_type!(timestamp_without_precision(
    kingbase_oracle,
    "TIMESTAMP",
    ColumnType::DateTime,
    Value::null_datetime(),
    Value::datetime(
        chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    )
));

test_type!(timestamp_tz(
    kingbase_oracle,
    "TIMESTAMP(6) WITH TIME ZONE",
    ColumnType::DateTime,
    (Value::null_datetime(), Value::null_datetime()),
    (
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
        .with_native_column_type(Some("TimestampTz")),
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
    )
));

test_type!(timestamp_tz_without_precision(
    kingbase_oracle,
    "TIMESTAMP WITH TIME ZONE",
    ColumnType::DateTime,
    (Value::null_datetime(), Value::null_datetime()),
    (
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
        .with_native_column_type(Some("TimestampTz")),
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
    )
));

test_type!(timestamp_local_tz(
    kingbase_oracle,
    "TIMESTAMP(6) WITH LOCAL TIME ZONE",
    ColumnType::DateTime,
    (Value::null_datetime(), Value::null_datetime()),
    (
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
        .with_native_column_type(Some("TimestampLocalTz")),
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
    )
));

test_type!(timestamp_local_tz_without_precision(
    kingbase_oracle,
    "TIMESTAMP WITH LOCAL TIME ZONE",
    ColumnType::DateTime,
    (Value::null_datetime(), Value::null_datetime()),
    (
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
        .with_native_column_type(Some("TimestampLocalTz")),
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
    )
));
