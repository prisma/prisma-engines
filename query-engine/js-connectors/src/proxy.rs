use core::panic;
use std::str::FromStr;

use crate::async_js_function::AsyncJsFunction;
use crate::conversion::JSArg;
use crate::transaction::JsTransaction;
use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};
use napi::{JsObject, JsString};
use napi_derive::napi;
use quaint::connector::{IsolationLevel, ResultSet as QuaintResultSet};
use quaint::Value as QuaintValue;

// TODO(jkomyno): import these 3rd-party crates from the `quaint-core` crate.
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use chrono::{NaiveDate, NaiveTime};

/// Proxy is a struct wrapping a javascript object that exhibits basic primitives for
/// querying and executing SQL (i.e. a client connector). The Proxy uses NAPI ThreadSafeFunction to
/// invoke the code within the node runtime that implements the client connector.
pub(crate) struct CommonProxy {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: AsyncJsFunction<Query, JSResultSet>,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: AsyncJsFunction<Query, u32>,

    /// Return the flavour for this driver.
    pub(crate) flavour: String,
}

/// This is a JS proxy for accessing the methods specific to top level
/// JS driver objects
pub(crate) struct DriverProxy {
    start_transaction: AsyncJsFunction<Option<String>, JsTransaction>,
}
/// This a JS proxy for accessing the methods, specific
/// to JS transaction objects
pub(crate) struct TransactionProxy {
    /// commit transaction
    commit: AsyncJsFunction<(), ()>,

    /// rollback transcation
    rollback: AsyncJsFunction<(), ()>,
}

/// This result set is more convenient to be manipulated from both Rust and NodeJS.
/// Quaint's version of ResultSet is:
///
/// pub struct ResultSet {
///     pub(crate) columns: Arc<Vec<String>>,
///     pub(crate) rows: Vec<Vec<Value<'static>>>,
///     pub(crate) last_insert_id: Option<u64>,
/// }
///
/// If we used this ResultSet would we would have worse ergonomics as quaint::Value is a structured
/// enum and cannot be used directly with the #[napi(Object)] macro. Thus requiring us to implement
/// the FromNapiValue and ToNapiValue traits for quaint::Value, and use a different custom type
/// representing the Value in javascript.
///
#[napi(object)]
#[derive(Debug)]
pub struct JSResultSet {
    pub column_types: Vec<ColumnType>,
    pub column_names: Vec<String>,
    // Note this might be encoded differently for performance reasons
    pub rows: Vec<Vec<serde_json::Value>>,
    pub last_insert_id: Option<String>,
}

impl JSResultSet {
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

#[napi]
#[derive(Debug)]
pub enum ColumnType {
    // [PLANETSCALE_TYPE] (MYSQL_TYPE) -> [TypeScript example]
    /// The following PlanetScale type IDs are mapped into Int32:
    /// - INT8 (TINYINT) -> e.g. `127`
    /// - INT16 (SMALLINT) -> e.g. `32767`
    /// - INT24 (MEDIUMINT) -> e.g. `8388607`
    /// - INT32 (INT) -> e.g. `2147483647`
    Int32,

    /// The following PlanetScale type IDs are mapped into Int64:
    /// - INT64 (BIGINT) -> e.g. `"9223372036854775807"` (String-encoded)
    Int64,

    /// The following PlanetScale type IDs are mapped into Float:
    /// - FLOAT32 (FLOAT) -> e.g. `3.402823466`
    Float,

    /// The following PlanetScale type IDs are mapped into Double:
    /// - FLOAT64 (DOUBLE) -> e.g. `1.7976931348623157`
    Double,

    /// The following PlanetScale type IDs are mapped into Numeric:
    /// - DECIMAL (DECIMAL) -> e.g. `"99999999.99"` (String-encoded)
    Numeric,

    /// The following PlanetScale type IDs are mapped into Boolean:
    /// - BOOLEAN (BOOLEAN) -> e.g. `1`
    Boolean,

    /// The following PlanetScale type IDs are mapped into Char:
    /// - CHAR (CHAR) -> e.g. `"c"` (String-encoded)
    Char,

    /// The following PlanetScale type IDs are mapped into Text:
    /// - TEXT (TEXT) -> e.g. `"foo"` (String-encoded)
    /// - VARCHAR (VARCHAR) -> e.g. `"foo"` (String-encoded)
    Text,

    /// The following PlanetScale type IDs are mapped into Date:
    /// - DATE (DATE) -> e.g. `"2023-01-01"` (String-encoded, yyyy-MM-dd)
    Date,

    /// The following PlanetScale type IDs are mapped into Time:
    /// - TIME (TIME) -> e.g. `"23:59:59"` (String-encoded, HH:mm:ss)
    Time,

    /// The following PlanetScale type IDs are mapped into DateTime:
    /// - DATETIME (DATETIME) -> e.g. `"2023-01-01 23:59:59"` (String-encoded, yyyy-MM-dd HH:mm:ss)
    /// - TIMESTAMP (TIMESTAMP) -> e.g. `"2023-01-01 23:59:59"` (String-encoded, yyyy-MM-dd HH:mm:ss)
    DateTime,

    /// The following PlanetScale type IDs are mapped into Json:
    /// - JSON (JSON) -> e.g. `"{\"key\": \"value\"}"` (String-encoded)
    Json,

    /// The following PlanetScale type IDs are mapped into Enum:
    /// - ENUM (ENUM) -> e.g. `"foo"` (String-encoded)
    Enum,

    /// The following PlanetScale type IDs are mapped into Bytes:
    /// - BLOB (BLOB) -> e.g. `"\u0012"` (String-encoded)
    /// - VARBINARY (VARBINARY) -> e.g. `"\u0012"` (String-encoded)
    /// - BINARY (BINARY) -> e.g. `"\u0012"` (String-encoded)
    /// - GEOMETRY (GEOMETRY) -> e.g. `"\u0012"` (String-encoded)
    Bytes,

    /// The following PlanetScale type IDs are mapped into Set:
    /// - SET (SET) -> e.g. `"foo,bar"` (String-encoded, comma-separated)
    /// This is currently unhandled, and will panic if encountered.
    Set,
}

#[napi(object)]
#[derive(Debug)]
pub struct Query {
    pub sql: String,
    pub args: Vec<JSArg>,
}

/// Handle data-type conversion from a JSON value to a Quaint value.
/// This is used for most data types, except those that require connector-specific handling, e.g., `ColumnType::Boolean`.
fn js_value_to_quaint(
    json_value: serde_json::Value,
    column_type: ColumnType,
    column_name: &str,
) -> QuaintValue<'static> {
    //  Note for the future: it may be worth revisiting how much bloat so many panics with different static
    // strings add to the compiled artefact, and in case we should come up with a restricted set of panic
    // messages, or even find a way of removing them altogether.
    // println!("js_value_to_quaint: {} / {:?}", json_value, column_type);
    match column_type {
        ColumnType::Int32 => match json_value {
            serde_json::Value::Number(n) => {
                // n.as_i32() is not implemented, so we need to downcast from i64 instead
                QuaintValue::int32(n.as_i64().expect("number must be an i32") as i32)
            }
            serde_json::Value::Null => QuaintValue::Int32(None),
            mismatch => panic!("Expected an i32 number in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Int64 => match json_value {
            serde_json::Value::String(s) => {
                let n = s.parse::<i64>().expect("string-encoded number must be an i64");
                QuaintValue::int64(n)
            }
            serde_json::Value::Null => QuaintValue::Int64(None),
            mismatch => panic!("Expected a string in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Float => match json_value {
            // n.as_f32() is not implemented, so we need to downcast from f64 instead.
            // We assume that the JSON value is a valid f32 number, but we check for overflows anyway.
            serde_json::Value::Number(n) => QuaintValue::float(f64_to_f32(n.as_f64().expect("number must be a f64"))),
            serde_json::Value::Null => QuaintValue::Float(None),
            mismatch => panic!("Expected a f32 number in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Double => match json_value {
            serde_json::Value::Number(n) => QuaintValue::double(n.as_f64().expect("number must be a f64")),
            serde_json::Value::Null => QuaintValue::Double(None),
            mismatch => panic!("Expected a f64 number in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Numeric => match json_value {
            serde_json::Value::String(s) => {
                let decimal = BigDecimal::from_str(&s).expect("invalid numeric value");
                QuaintValue::numeric(decimal)
            }
            serde_json::Value::Null => QuaintValue::Numeric(None),
            mismatch => panic!(
                "Expected a string-encoded number in column {}, found {}",
                column_name, mismatch
            ),
        },
        ColumnType::Boolean => match json_value {
            serde_json::Value::Bool(b) => QuaintValue::boolean(b),
            serde_json::Value::Null => QuaintValue::Boolean(None),
            mismatch => panic!("Expected a boolean in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Char => match json_value {
            serde_json::Value::String(s) => QuaintValue::Char(s.chars().next()),
            serde_json::Value::Null => QuaintValue::Char(None),
            mismatch => panic!("Expected a string in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Text => match json_value {
            serde_json::Value::String(s) => QuaintValue::text(s),
            serde_json::Value::Null => QuaintValue::Text(None),
            mismatch => panic!("Expected a string in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Date => match json_value {
            serde_json::Value::String(s) => {
                let date = NaiveDate::parse_from_str(&s, "%Y-%m-%d").expect("Expected a date string");
                QuaintValue::date(date)
            }
            serde_json::Value::Null => QuaintValue::Date(None),
            mismatch => panic!("Expected a string in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Time => match json_value {
            serde_json::Value::String(s) => {
                let time = NaiveTime::parse_from_str(&s, "%H:%M:%S").expect("Expected a time string");
                QuaintValue::time(time)
            }
            serde_json::Value::Null => QuaintValue::Time(None),
            mismatch => panic!("Expected a string in column {}, found {}", column_name, mismatch),
        },
        ColumnType::DateTime => match json_value {
            serde_json::Value::String(s) => {
                let datetime = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f")
                    .unwrap_or_else(|_| panic!("Expected a datetime string, found {:?}", &s));
                let datetime: DateTime<Utc> = DateTime::from_utc(datetime, Utc);
                QuaintValue::datetime(datetime)
            }
            serde_json::Value::Null => QuaintValue::DateTime(None),
            mismatch => panic!("Expected a string in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Json => match json_value {
            serde_json::Value::Null => QuaintValue::Json(None),
            json => QuaintValue::json(json),
        },
        ColumnType::Enum => match json_value {
            serde_json::Value::String(s) => QuaintValue::enum_variant(s),
            serde_json::Value::Null => QuaintValue::Enum(None),
            mismatch => panic!("Expected a string in column {}, found {}", column_name, mismatch),
        },
        ColumnType::Bytes => match json_value {
            serde_json::Value::String(s) => QuaintValue::Bytes(Some(s.into_bytes().into())),
            serde_json::Value::Null => QuaintValue::Bytes(None),
            mismatch => panic!("Expected a string in column {}, found {}", column_name, mismatch),
        },
        unimplemented => {
            todo!("support column type {:?} in column {}", unimplemented, column_name)
        }
    }
}

impl From<JSResultSet> for QuaintResultSet {
    fn from(js_result_set: JSResultSet) -> Self {
        let JSResultSet {
            rows,
            column_names,
            column_types,
            last_insert_id,
        } = js_result_set;

        let mut quaint_rows = Vec::with_capacity(rows.len());

        for row in rows {
            let mut quaint_row = Vec::with_capacity(column_types.len());

            for (i, row) in row.into_iter().enumerate() {
                let column_type = column_types[i];
                let column_name = column_names[i].as_str();

                quaint_row.push(js_value_to_quaint(row, column_type, column_name));
            }

            quaint_rows.push(quaint_row);
        }

        let last_insert_id = last_insert_id.and_then(|id| id.parse::<u64>().ok());
        let mut quaint_result_set = QuaintResultSet::new(column_names, quaint_rows);

        // Not a fan of this (extracting the `Some` value from an `Option` and pass it to a method that creates a new `Some` value),
        // but that's Quaint's ResultSet API and that's how the MySQL connector does it.
        // Sqlite, on the other hand, uses a `last_insert_id.unwrap_or(0)` approach.
        if let Some(last_insert_id) = last_insert_id {
            quaint_result_set.set_last_insert_id(last_insert_id);
        }

        quaint_result_set
    }
}

impl CommonProxy {
    pub fn new(object: &JsObject) -> napi::Result<Self> {
        let flavour: JsString = object.get_named_property("flavour")?;

        Ok(Self {
            query_raw: object.get_named_property("queryRaw")?,
            execute_raw: object.get_named_property("executeRaw")?,
            flavour: flavour.into_utf8()?.as_str()?.to_owned(),
        })
    }

    pub async fn query_raw(&self, params: Query) -> quaint::Result<JSResultSet> {
        self.query_raw.call(params).await
    }

    pub async fn execute_raw(&self, params: Query) -> quaint::Result<u32> {
        self.execute_raw.call(params).await
    }
}

impl DriverProxy {
    pub fn new(js_connector: &JsObject) -> napi::Result<Self> {
        Ok(Self {
            start_transaction: js_connector.get_named_property("startTransaction")?,
        })
    }

    pub async fn start_transaction(
        &self,
        isolation_level: Option<IsolationLevel>,
    ) -> quaint::Result<Box<JsTransaction>> {
        let tx = self
            .start_transaction
            .call(isolation_level.map(|l| l.to_string()))
            .await?;
        Ok(Box::new(tx))
    }
}

impl TransactionProxy {
    pub fn new(js_transaction: &JsObject) -> napi::Result<Self> {
        let commit = js_transaction.get_named_property("commit")?;
        let rollback = js_transaction.get_named_property("rollback")?;

        Ok(Self { commit, rollback })
    }

    pub async fn commit(&self) -> quaint::Result<()> {
        self.commit.call(()).await
    }
    pub async fn rollback(&self) -> quaint::Result<()> {
        self.rollback.call(()).await
    }
}

/// Coerce a `f64` to a `f32`, asserting that the conversion is lossless.
/// Note that, when overflow occurs during conversion, the result is `infinity`.
fn f64_to_f32(x: f64) -> f32 {
    let y = x as f32;

    assert_eq!(x.is_finite(), y.is_finite(), "f32 overflow during conversion");

    y
}

#[cfg(test)]
mod proxy_test {
    use num_bigint::BigInt;
    use serde_json::json;

    use super::*;

    #[track_caller]
    fn test_null(quaint_none: QuaintValue, column_type: ColumnType) {
        let json_value = serde_json::Value::Null;
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, quaint_none);
    }

    #[test]
    fn js_value_int32_to_quaint() {
        let column_type = ColumnType::Int32;

        // null
        test_null(QuaintValue::Int32(None), column_type);

        // 0
        let n: i32 = 0;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Int32(Some(n)));

        // max
        let n: i32 = i32::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Int32(Some(n)));

        // min
        let n: i32 = i32::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Int32(Some(n)));
    }

    #[test]
    fn js_value_int64_to_quaint() {
        let column_type = ColumnType::Int64;

        // null
        test_null(QuaintValue::Int64(None), column_type);

        // 0
        let n: i64 = 0;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Int64(Some(n)));

        // max
        let n: i64 = i64::MAX;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Int64(Some(n)));

        // min
        let n: i64 = i64::MIN;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Int64(Some(n)));
    }

    #[test]
    fn js_value_float_to_quaint() {
        let column_type = ColumnType::Float;

        // null
        test_null(QuaintValue::Float(None), column_type);

        // 0
        let n: f32 = 0.0;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Float(Some(n)));

        // max
        let n: f32 = f32::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Float(Some(n)));

        // min
        let n: f32 = f32::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Float(Some(n)));
    }

    #[test]
    fn js_value_double_to_quaint() {
        let column_type = ColumnType::Double;

        // null
        test_null(QuaintValue::Double(None), column_type);

        // 0
        let n: f64 = 0.0;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Double(Some(n)));

        // max
        let n: f64 = f64::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Double(Some(n)));

        // min
        let n: f64 = f64::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Double(Some(n)));
    }

    #[test]
    fn js_value_numeric_to_quaint() {
        let column_type = ColumnType::Numeric;

        // null
        test_null(QuaintValue::Numeric(None), column_type);

        let n_as_string = "1234.99";
        let decimal = BigDecimal::new(BigInt::parse_bytes(b"123499", 10).unwrap(), 2);

        let json_value = serde_json::Value::String(n_as_string.into());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Numeric(Some(decimal)));

        let n_as_string = "1234.999999";
        let decimal = BigDecimal::new(BigInt::parse_bytes(b"1234999999", 10).unwrap(), 6);

        let json_value = serde_json::Value::String(n_as_string.into());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Numeric(Some(decimal)));
    }

    #[test]
    fn js_value_boolean_to_quaint() {
        let column_type = ColumnType::Boolean;

        // null
        test_null(QuaintValue::Boolean(None), column_type);

        // true
        let bool_val = true;
        let json_value = serde_json::Value::Bool(bool_val);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Boolean(Some(bool_val)));

        // false
        let bool_val = false;
        let json_value = serde_json::Value::Bool(bool_val);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Boolean(Some(bool_val)));
    }

    #[test]
    fn js_value_char_to_quaint() {
        let column_type = ColumnType::Char;

        // null
        test_null(QuaintValue::Char(None), column_type);

        let c = 'c';
        let json_value = serde_json::Value::String(c.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Char(Some(c)));
    }

    #[test]
    fn js_value_text_to_quaint() {
        let column_type = ColumnType::Text;

        // null
        test_null(QuaintValue::Text(None), column_type);

        let s = "some text";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Text(Some(s.into())));
    }

    #[test]
    fn js_value_date_to_quaint() {
        let column_type = ColumnType::Date;

        // null
        test_null(QuaintValue::Date(None), column_type);

        let s = "2023-01-01";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        assert_eq!(quaint_value, QuaintValue::Date(Some(date)));
    }

    #[test]
    fn js_value_time_to_quaint() {
        let column_type = ColumnType::Time;

        // null
        test_null(QuaintValue::Time(None), column_type);

        let s = "23:59:59";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        let time: NaiveTime = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
        assert_eq!(quaint_value, QuaintValue::Time(Some(time)));
    }

    #[test]
    fn js_value_datetime_to_quaint() {
        let column_type = ColumnType::DateTime;

        // null
        test_null(QuaintValue::DateTime(None), column_type);

        let s = "2023-01-01 23:59:59.415";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_milli_opt(23, 59, 59, 415)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value, QuaintValue::DateTime(Some(datetime)));

        let s = "2023-01-01 23:59:59.123456";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_micro_opt(23, 59, 59, 123_456)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value, QuaintValue::DateTime(Some(datetime)));

        let s = "2023-01-01 23:59:59";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_milli_opt(23, 59, 59, 0)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value, QuaintValue::DateTime(Some(datetime)));
    }

    #[test]
    fn js_value_json_to_quaint() {
        let column_type = ColumnType::Json;

        // null
        test_null(QuaintValue::Json(None), column_type);

        let json = json!({
            "key": "value",
            "nested": [
                true,
                false,
                1,
                null
            ]
        });
        let json_value = json.clone();
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Json(Some(json.clone())));
    }

    #[test]
    fn js_value_enum_to_quaint() {
        let column_type = ColumnType::Enum;

        // null
        test_null(QuaintValue::Enum(None), column_type);

        let s = "some enum variant";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");
        assert_eq!(quaint_value, QuaintValue::Enum(Some(s.into())));
    }
}
