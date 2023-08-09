use core::panic;
use std::str::FromStr;

use quaint::connector::ResultSet as QuaintResultSet;
use quaint::Value as QuaintValue;

// TODO(jkomyno): import these 3rd-party crates from the `quaint-core` crate.
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use chrono::{NaiveDate, NaiveTime};

use async_trait::async_trait;

use js_sys::{Function as JsFunction, Promise as JsPromise};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

/// Proxy is a struct wrapping a javascript object that exhibits basic primitives for
/// querying and executing SQL (i.e. a client connector). The Proxy uses NAPI ThreadSafeFunction to
/// invoke the code within the node runtime that implements the client connector.
#[derive(Clone)]
#[wasm_bindgen]
pub struct Proxy {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: JsFunction,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: JsFunction,

    /// Return the version of the underlying database, queried directly from the
    /// source.
    version: JsFunction,

    /// Closes the underlying database connection.
    #[allow(dead_code)]
    close: JsFunction,

    /// Return true iff the underlying database connection is healthy.
    #[allow(dead_code)]
    is_healthy: JsFunction,

    /// Return the flavor for this driver.
    #[allow(dead_code)]
    pub(crate) flavor: String,
}

#[wasm_bindgen]
impl Proxy {
    #[wasm_bindgen(constructor)]
    pub fn new(
        query_raw: JsFunction,
        execute_raw: JsFunction,
        version: JsFunction,
        close: JsFunction,
        is_healthy: JsFunction,
        flavor: String,
    ) -> Proxy {
        Proxy {
            query_raw,
            execute_raw,
            version,
            close,
            is_healthy,
            flavor,
        }
    }
}

type Result<T> = std::result::Result<T, js_sys::Error>;

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
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Copy, Clone, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
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

#[derive(Debug, Serialize)]
pub struct Query {
    pub sql: String,
    pub args: Vec<serde_json::Value>,
}

/// Coerce a `f64` to a `f32`, asserting that the conversion is lossless.
/// Note that, when overflow occurs during conversion, the result is `infinity`.
fn f64_to_f32(x: f64) -> f32 {
    let y = x as f32;

    assert_eq!(x.is_finite(), y.is_finite(), "f32 overflow during conversion");

    y
}

/// Handle data-type conversion from a JSON value to a Quaint value.
/// This is used for most data types, except those that require connector-specific handling, e.g., `ColumnType::Boolean`.
/// In the future, after https://github.com/prisma/team-orm/issues/257, every connector-specific handling should be moved
/// out of Rust and into TypeScript.
fn js_value_to_quaint(json_value: serde_json::Value, column_type: ColumnType) -> QuaintValue<'static> {
    //  Note for the future: it may be worth revisiting how much bloat so many panics with different static
    // strings add to the compiled artefact, and in case we should come up with a restricted set of panic
    // messages, or even find a way of removing them altogether.
    match column_type {
        ColumnType::Int32 => match json_value {
            serde_json::Value::Number(n) => {
                // n.as_i32() is not implemented, so we need to downcast from i64 instead
                QuaintValue::int32(n.as_i64().expect("number must be an i32") as i32)
            }
            serde_json::Value::Null => QuaintValue::Int32(None),
            mismatch => panic!("Expected an i32 number, found {:?}", mismatch),
        },
        ColumnType::Int64 => match json_value {
            serde_json::Value::String(s) => {
                let n = s.parse::<i64>().expect("string-encoded number must be an i64");
                QuaintValue::int64(n)
            }
            serde_json::Value::Null => QuaintValue::Int64(None),
            mismatch => panic!("Expected a string, found {:?}", mismatch),
        },
        ColumnType::Float => match json_value {
            // n.as_f32() is not implemented, so we need to downcast from f64 instead.
            // We assume that the JSON value is a valid f32 number, but we check for overflows anyway.
            serde_json::Value::Number(n) => QuaintValue::float(f64_to_f32(n.as_f64().expect("number must be a f64"))),
            serde_json::Value::Null => QuaintValue::Float(None),
            mismatch => panic!("Expected a f32 number, found {:?}", mismatch),
        },
        ColumnType::Double => match json_value {
            serde_json::Value::Number(n) => QuaintValue::double(n.as_f64().expect("number must be a f64")),
            serde_json::Value::Null => QuaintValue::Double(None),
            mismatch => panic!("Expected a f64 number, found {:?}", mismatch),
        },
        ColumnType::Numeric => match json_value {
            serde_json::Value::String(s) => {
                let decimal = BigDecimal::from_str(&s).expect("invalid numeric value");
                QuaintValue::numeric(decimal)
            }
            serde_json::Value::Null => QuaintValue::Numeric(None),
            mismatch => panic!("Expected a string-encoded number, found {:?}", mismatch),
        },
        ColumnType::Boolean => match json_value {
            serde_json::Value::Bool(b) => QuaintValue::boolean(b),
            serde_json::Value::Null => QuaintValue::Boolean(None),
            mismatch => panic!("Expected a boolean, found {:?}", mismatch),
        },
        ColumnType::Char => match json_value {
            serde_json::Value::String(s) => QuaintValue::Char(s.chars().next()),
            serde_json::Value::Null => QuaintValue::Char(None),
            mismatch => panic!("Expected a string, found {:?}", mismatch),
        },
        ColumnType::Text => match json_value {
            serde_json::Value::String(s) => QuaintValue::text(s),
            serde_json::Value::Null => QuaintValue::Text(None),
            mismatch => panic!("Expected a string, found {:?}", mismatch),
        },
        ColumnType::Date => match json_value {
            serde_json::Value::String(s) => {
                let date = NaiveDate::parse_from_str(&s, "%Y-%m-%d").expect("Expected a date string");
                QuaintValue::date(date)
            }
            serde_json::Value::Null => QuaintValue::Date(None),
            mismatch => panic!("Expected a string, found {:?}", mismatch),
        },
        ColumnType::Time => match json_value {
            serde_json::Value::String(s) => {
                let time = NaiveTime::parse_from_str(&s, "%H:%M:%S").expect("Expected a time string");
                QuaintValue::time(time)
            }
            serde_json::Value::Null => QuaintValue::Time(None),
            mismatch => panic!("Expected a string, found {:?}", mismatch),
        },
        ColumnType::DateTime => match json_value {
            serde_json::Value::String(s) => {
                let datetime = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_else(|_| panic!("Expected a datetime string, found {:?}", &s));
                let datetime: DateTime<Utc> = DateTime::from_utc(datetime, Utc);
                QuaintValue::datetime(datetime)
            }
            serde_json::Value::Null => QuaintValue::DateTime(None),
            mismatch => panic!("Expected a string, found {:?}", mismatch),
        },
        ColumnType::Json => match json_value {
            serde_json::Value::Null => QuaintValue::Json(None),
            json => QuaintValue::json(json),
        },
        ColumnType::Enum => match json_value {
            serde_json::Value::String(s) => QuaintValue::enum_variant(s),
            serde_json::Value::Null => QuaintValue::Enum(None),
            mismatch => panic!("Expected a string, found {:?}", mismatch),
        },
        ColumnType::Bytes => match json_value {
            serde_json::Value::String(s) => QuaintValue::Bytes(Some(s.into_bytes().into())),
            serde_json::Value::Null => QuaintValue::Bytes(None),
            mismatch => panic!("Expected a string, found {:?}", mismatch),
        },
        unimplemented => {
            todo!("support column type: Column: {:?}", unimplemented)
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

        let quaint_rows = rows
            .into_iter()
            .map(move |row| {
                column_types
                    .iter()
                    .zip(row)
                    .map(|(column_type, value)| js_value_to_quaint(value, *column_type))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

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

#[async_trait(?Send)]
trait JsAsyncFunc {
    async fn call1_async<T, R>(&self, arg1: T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned;

    fn call0_sync<R>(&self) -> Result<R>
    where
        R: DeserializeOwned;
}

#[async_trait(?Send)]
impl JsAsyncFunc for JsFunction {
    async fn call1_async<T, R>(&self, arg1: T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let arg1 = serde_wasm_bindgen::to_value(&arg1).map_err(|err| js_sys::Error::new(&err.to_string()))?;
        let promise = self.call1(&JsValue::null(), &arg1)?;
        let future = wasm_bindgen_futures::JsFuture::from(JsPromise::from(promise));
        let value = future.await?;
        serde_wasm_bindgen::from_value(value).map_err(|err| js_sys::Error::new(&err.to_string()))
    }

    fn call0_sync<R>(&self) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let value = self.call0(&JsValue::null())?;
        serde_wasm_bindgen::from_value(value).map_err(|err| js_sys::Error::new(&err.to_string()))
    }
}

impl Proxy {
    pub async fn query_raw(&self, params: Query) -> Result<JSResultSet> {
        let value = self.query_raw.call1_async::<_, JSResultSet>(params).await?;
        Ok(value)
    }

    pub async fn execute_raw(&self, params: Query) -> Result<u32> {
        let value = self.execute_raw.call1_async::<_, f32>(params).await? as u32;
        Ok(value)
    }

    pub async fn version(&self) -> Result<Option<String>> {
        let version = self.version.call0_sync::<Option<String>>()?;
        Ok(version)
    }

    pub async fn close(&self) -> Result<()> {
        self.close.call0_sync::<()>()
    }

    pub fn is_healthy(&self) -> Result<bool> {
        // TODO: call `is_healthy` in a blocking fashion, returning its result as a boolean.
        unimplemented!();
    }
}
