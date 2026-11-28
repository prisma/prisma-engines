//! Real-connection type coverage for KingbaseES Oracle-compatible mode.
//!
//! These tests deliberately do not reuse `types/mysql.rs` or
//! `types/postgres.rs`. Oracle-mode types use different DDL names and
//! PostgreSQL-wire OIDs, and arrays are intentionally unsupported.

use crate::{connector::ColumnType, macros::assert_matching_value_and_column_type, tests::test_api::*};
use std::str::FromStr;

use crate::bigdecimal::BigDecimal;

#[test]
fn raw_datetime_bindings_preserve_instants_without_changing_session_time_zone() -> crate::Result<()> {
    use crate::{ast::Value, connector::Queryable, tests::test_api::kingbase_oracle::CONN_STR};

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            for timezone in ["UTC", "Asia/Shanghai", "America/New_York"] {
                let mut url = url::Url::parse(CONN_STR.as_str()).unwrap();
                url.query_pairs_mut()
                    .append_pair("options", &format!("-ctimezone={timezone}"));
                let mut setup = KingbaseOracle::new(url.as_str()).await?;
                let table = setup.create_temp_table("value TIMESTAMP(6) WITH TIME ZONE").await?;
                let conn = setup.conn();
                for input in ["2026-01-02T00:00:00.123456Z", "2026-07-02T08:00:00.123456+08:00"] {
                    let instant = chrono::DateTime::parse_from_rfc3339(input)
                        .unwrap()
                        .with_timezone(&chrono::Utc);
                    let params = [Value::datetime(instant)];
                    let row = conn
                        .query_raw_typed("SELECT $1 AS value", &params)
                        .await?
                        .into_single()?;
                    assert_eq!(row["value"], Value::datetime(instant));
                    // Without a model hint, an explicit SQL cast to a plain
                    // timestamp follows the database's normal zone conversion.
                    let plain = conn
                        .query_raw_typed("SELECT CAST($1 AS TIMESTAMP) AS value", &params)
                        .await?
                        .into_single()?;
                    let literal = conn
                        .query_raw(
                            &format!(
                                "SELECT CAST(CAST('{}' AS TIMESTAMP WITH TIME ZONE) AS TIMESTAMP) AS value",
                                instant.to_rfc3339()
                            ),
                            &[],
                        )
                        .await?
                        .into_single()?;
                    assert_eq!(plain["value"], literal["value"], "{timezone} / {input}");
                    for typed in [false, true] {
                        let params = [Value::datetime(instant)];
                        let sql = "SELECT CAST($1 AS TIMESTAMP WITH TIME ZONE) AS value";
                        let row = if typed {
                            conn.query_raw_typed(sql, &params).await?
                        } else {
                            conn.query_raw(sql, &params).await?
                        }
                        .into_single()?;
                        assert_eq!(
                            row["value"],
                            Value::datetime(instant),
                            "{timezone} / {input} / typed={typed}"
                        );
                        let sql = format!("INSERT INTO \"{table}\" VALUES($1)");
                        if typed {
                            conn.execute_raw_typed(&sql, &params).await?;
                        } else {
                            conn.execute_raw(&sql, &params).await?;
                        }
                        let row = conn
                            .query_raw(
                                &format!(
                                    "SELECT value, CAST(EXTRACT(EPOCH FROM value) AS TEXT) AS seconds FROM \"{table}\""
                                ),
                                &[],
                            )
                            .await?
                            .into_single()?;
                        assert_eq!(row["value"], Value::datetime(instant));
                        let seconds: bigdecimal::BigDecimal = row["seconds"].as_str().unwrap().parse().unwrap();
                        let expected = bigdecimal::BigDecimal::from(instant.timestamp_micros())
                            / bigdecimal::BigDecimal::from(1_000_000);
                        assert_eq!(seconds, expected, "{timezone} / {input} / typed={typed}");
                        conn.raw_cmd(&format!("DELETE FROM \"{table}\"")).await?;
                    }
                }
                let row = conn
                    .query_raw_typed(
                        "SELECT CAST($1 AS TIMESTAMP WITH TIME ZONE) AS value",
                        &[Value::null_datetime()],
                    )
                    .await?
                    .into_single()?;
                assert!(row["value"].is_null());
                let row = conn
                    .query_raw("SELECT current_setting('TimeZone') AS timezone", &[])
                    .await?
                    .into_single()?;
                assert_eq!(row["timezone"], Value::text(timezone));
            }
            Ok(())
        })
}

#[test]
fn timestamp_roundtrips_preserve_session_time_zone() -> crate::Result<()> {
    use crate::{ast::Value, connector::Queryable, tests::test_api::kingbase_oracle::CONN_STR};
    use chrono::Timelike;

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            for timezone in ["UTC", "Asia/Shanghai", "America/New_York"] {
                let mut url = url::Url::parse(CONN_STR.as_str()).unwrap();
                url.query_pairs_mut()
                    .append_pair("options", &format!("-ctimezone={timezone}"));
                let mut setup = KingbaseOracle::new(url.as_str()).await?;
                let conn = setup.conn();
                let timezone_sql = "SELECT current_setting('TimeZone') AS timezone";
                let result = conn.query_raw(timezone_sql, &[]).await?.into_single()?;
                assert_eq!(result.at(0), Some(&Value::text(timezone)));

                let table = setup
                    .create_temp_table(
                        "d DATE, plain TIMESTAMP(6), zoned TIMESTAMP(6) WITH TIME ZONE, localized TIMESTAMP(6) WITH LOCAL TIME ZONE",
                    )
                    .await?;
                let conn = setup.conn();
                for input in [
                    "2026-01-02T00:00:00.123456Z",
                    "2026-01-02T08:00:00.123456+08:00",
                    "2026-07-02T05:30:00.123456+05:30",
                    "2026-03-08T02:30:00.123456Z",
                ] {
                    let instant = chrono::DateTime::parse_from_rfc3339(input)
                        .unwrap()
                        .with_timezone(&chrono::Utc);
                    let params = ["Date", "Timestamp", "TimestampTz", "TimestampLocalTz"]
                        .map(|native_type| Value::datetime(instant).with_native_column_type(Some(native_type)));
                    conn.execute_raw_typed(
                        &format!("INSERT INTO \"{table}\" VALUES ($1, $2, $3, $4)"),
                        &params,
                    )
                    .await?;
                    let row = conn
                        .query_raw(&format!("SELECT * FROM \"{table}\""), &[])
                        .await?
                        .into_single()?;
                    assert_eq!(row.at(0), Some(&Value::datetime(instant.with_nanosecond(0).unwrap())));
                    for index in 1..4 {
                        assert_eq!(row.at(index), Some(&Value::datetime(instant)), "{timezone}: {input}");
                    }
                    conn.raw_cmd(&format!("DELETE FROM \"{table}\"")).await?;
                }
                let result = conn.query_raw(timezone_sql, &[]).await?.into_single()?;
                assert_eq!(result.at(0), Some(&Value::text(timezone)));
            }
            Ok(())
        })
}

test_type!(tinyint(
    kingbase_oracle,
    "TINYINT",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(-128),
    Value::int32(-1),
    Value::int32(0),
    Value::int32(1),
    Value::int32(127)
));

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
    (Value::null_datetime(), Value::null_datetime()),
    (
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
        .with_native_column_type(Some("Date")),
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
    )
));

test_type!(timestamp(
    kingbase_oracle,
    "TIMESTAMP(6)",
    ColumnType::DateTime,
    (Value::null_datetime(), Value::null_datetime()),
    (
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
        .with_native_column_type(Some("Timestamp")),
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
    )
));

test_type!(timestamp_without_precision(
    kingbase_oracle,
    "TIMESTAMP",
    ColumnType::DateTime,
    (Value::null_datetime(), Value::null_datetime()),
    (
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
        .with_native_column_type(Some("Timestamp")),
        Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2025-01-02T03:04:05.123456Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
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

test_type!(char_byte(
    kingbase_oracle,
    "CHAR(8 BYTE)",
    ColumnType::Text,
    (Value::null_text(), Value::null_text()),
    (Value::text("abc"), Value::text("abc     ")),
    (Value::text("中文ab"), Value::text("中文ab"))
));

test_type!(varchar2_byte(
    kingbase_oracle,
    "VARCHAR2(8 BYTE)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("中文ab")
));

test_type!(binary_integer(
    kingbase_oracle,
    "BINARY_INTEGER",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i32::MIN),
    Value::int32(0),
    Value::int32(i32::MAX)
));

test_type!(urowid(
    kingbase_oracle,
    "UROWID(4000)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("AAAAAAAAAAABAAAAAAAAAAA")
));

#[tokio::test]
async fn rowid_native_parameters_roundtrip_with_server_generated_identifiers() -> crate::Result<()> {
    use crate::{Value, prelude::Queryable, single::Quaint, tests::test_api::kingbase_oracle::CONN_STR};
    let conn = Quaint::new(CONN_STR.as_str()).await?;
    conn.raw_cmd("CREATE TEMPORARY TABLE rowid_codec_source (id INTEGER) WITH ROWID;\nINSERT INTO rowid_codec_source VALUES (1), (2);\nCREATE TEMPORARY TABLE rowid_codec_target (value ROWID UNIQUE)").await?;
    let generated = conn
        .query_raw("SELECT rowid, rowid::text AS expected FROM rowid_codec_source", &[])
        .await?;
    assert_eq!(generated.len(), 2);
    for row in generated {
        let expected = row["expected"].as_str().unwrap();
        assert_eq!(row["rowid"].as_str(), Some(expected));
        let value = Value::text(expected.to_owned()).with_native_column_type(Some("RowId"));
        for typed in [false, true] {
            let params = [value.clone()];
            let sql = "INSERT INTO rowid_codec_target VALUES ($1) RETURNING value";
            let result = if typed {
                conn.query_raw_typed(sql, &params).await?
            } else {
                conn.query_raw(sql, &params).await?
            };
            assert_eq!(result.into_single()?["value"].as_str(), Some(expected));
            conn.raw_cmd("DELETE FROM rowid_codec_target").await?;
        }
    }
    let null = conn.query_raw("SELECT CAST(NULL AS ROWID) AS value", &[]).await?;
    assert_eq!(null.into_single()?["value"], Value::null_text());
    let invalid = [Value::text("invalid-rowid").with_native_column_type(Some("RowId"))];
    assert!(conn.query_raw_typed("SELECT $1 AS value", &invalid).await.is_err());
    Ok(())
}
