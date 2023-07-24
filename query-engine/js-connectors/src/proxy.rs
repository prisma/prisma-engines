use core::panic;

use napi::bindgen_prelude::{FromNapiValue, Promise as JsPromise, ToNapiValue};
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};
use napi::JsObject;
use napi_derive::napi;
use quaint::connector::ResultSet as QuaintResultSet;
use quaint::Value as QuaintValue;

// TODO(jkomyno): import these 3rd-party crates from the `quaint-core` crate.
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use chrono::{NaiveDate, NaiveTime};

/// Proxy is a struct wrapping a javascript object that exhibits basic primitives for
/// querying and executing SQL (i.e. a client connector). The Proxy uses NAPI ThreadSafeFunction to
/// invoke the code within the node runtime that implements the client connector.
#[derive(Clone)]
pub struct Proxy {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: ThreadsafeFunction<Query, ErrorStrategy::Fatal>,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: ThreadsafeFunction<Query, ErrorStrategy::Fatal>,

    /// Return the version of the underlying database, queried directly from the
    /// source.
    version: ThreadsafeFunction<(), ErrorStrategy::Fatal>,

    /// Closes the underlying database connection.
    #[allow(dead_code)]
    close: ThreadsafeFunction<(), ErrorStrategy::Fatal>,

    /// Return true iff the underlying database connection is healthy.
    #[allow(dead_code)]
    is_healthy: ThreadsafeFunction<(), ErrorStrategy::Fatal>,
}

/// Reify creates a Rust proxy to access the JS driver passed in as a parameter.
pub fn reify(js_driver: JsObject) -> napi::Result<Proxy> {
    let query_raw = js_driver.get_named_property("queryRaw")?;
    let execute_raw = js_driver.get_named_property("executeRaw")?;
    let version = js_driver.get_named_property("version")?;
    let close = js_driver.get_named_property("close")?;
    let is_healthy = js_driver.get_named_property("isHealthy")?;

    let driver = Proxy {
        query_raw,
        execute_raw,
        version,
        close,
        is_healthy,
    };
    Ok(driver)
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
    ///   Note(jkomyno): TIMESTAMP is distinguished by DATETIME as it implies database-level UTC
    ///   conversions that don't happen with DATETIME values. Quaint does not support this distinction.
    ///   Can this lead to bugs we're not yet aware of?
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
    pub args: Vec<serde_json::Value>,
}

impl From<JSResultSet> for QuaintResultSet {
    fn from(mut js_result_set: JSResultSet) -> Self {
        // TODO: extract, todo: error rather than panic?
        let to_quaint_row = move |row: &mut Vec<serde_json::Value>| -> Vec<quaint::Value<'static>> {
            let mut res = Vec::with_capacity(row.len());

            for i in 0..row.len() {
                // Note(jkomyno): I suggest we avoid so many different panic messages.
                // They bloat the compiled code, making it heavier. I suggest we consider the type information
                // received from the JS connector safe, and eventually just let the code panic without a custom message here.
                // We can later infer which column type is not properly supported from the panic message's line and column anyway.
                match js_result_set.column_types[i] {
                    ColumnType::Int32 => match row.remove(0) {
                        serde_json::Value::Number(n) => {
                            // n.as_i32() is not implemented, so we need to downcast from i64 instead
                            res.push(QuaintValue::int32(n.as_i64().expect("number must be an i32") as i32))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Int32(None)),
                        mismatch => panic!("Expected an i32 number, found {:?}", mismatch),
                    },
                    ColumnType::Int64 => match row.remove(0) {
                        serde_json::Value::String(s) => {
                            let n = s.parse::<i64>().expect("string-encoded number must be an i64");
                            res.push(QuaintValue::int64(n))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Int64(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    ColumnType::Float => match row.remove(0) {
                        // n.as_f32() is not implemented, so we need to downcast from f64 instead
                        serde_json::Value::Number(n) => {
                            res.push(QuaintValue::float(n.as_f64().expect("number must be a f32") as f32))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Float(None)),
                        mismatch => panic!("Expected a f32 number, found {:?}", mismatch),
                    },
                    ColumnType::Double => match row.remove(0) {
                        serde_json::Value::Number(n) => {
                            res.push(QuaintValue::double(n.as_f64().expect("number must be a f64")))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Double(None)),
                        mismatch => panic!("Expected a f64 number, found {:?}", mismatch),
                    },
                    ColumnType::Numeric => match row.remove(0) {
                        serde_json::Value::String(s) => {
                            // Turn this into a BigInt value with an additional "scale" variable indicating the scale of 10.
                            // E.g., if s = "1234.99", s_as_bigint = 123499, s_scale = 2.
                            let (s_as_bigint, s_scale) = if let Some(dot) = s.find('.') {
                                let scale = s.len() - dot - 1;
                                let s = s.replace(".", "");
                                (
                                    num_bigint::BigInt::parse_bytes(s.as_bytes(), 10)
                                        .expect("string-encoded number must be a numeric"),
                                    scale as i64,
                                )
                            } else {
                                (
                                    num_bigint::BigInt::parse_bytes(s.as_bytes(), 10)
                                        .expect("string-encoded number must be a numeric"),
                                    0,
                                )
                            };
                            let decimal = BigDecimal::new(s_as_bigint, s_scale);
                            res.push(QuaintValue::numeric(decimal));
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Numeric(None)),
                        mismatch => panic!("Expected a string-encoded number, found {:?}", mismatch),
                    },
                    ColumnType::Boolean => match row.remove(0) {
                        serde_json::Value::Number(b) => {
                            res.push(QuaintValue::Boolean(b.as_u64().or(None).map(|b| b != 0)))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Boolean(None)),
                        mismatch => panic!("Expected a number, found {:?}", mismatch),
                    },
                    ColumnType::Char => match row.remove(0) {
                        serde_json::Value::String(s) if s.len() == 1 => res.push(QuaintValue::Char(s.chars().next())),
                        serde_json::Value::Null => res.push(QuaintValue::Char(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    ColumnType::Text => match row.remove(0) {
                        serde_json::Value::String(s) => res.push(QuaintValue::text(s)),
                        serde_json::Value::Null => res.push(QuaintValue::Text(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    ColumnType::Date => match row.remove(0) {
                        serde_json::Value::String(s) => {
                            let date = NaiveDate::parse_from_str(&s, "%Y-%m-%d").expect("Expected a date string");
                            res.push(QuaintValue::date(date))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Date(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    ColumnType::Time => match row.remove(0) {
                        serde_json::Value::String(s) => {
                            let time = NaiveTime::parse_from_str(&s, "%H:%M:%S").expect("Expected a time string");
                            res.push(QuaintValue::time(time))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Date(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    ColumnType::DateTime => match row.remove(0) {
                        serde_json::Value::String(s) => {
                            let datetime = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                                .expect(format!("Expected a datetime string, found {:?}", &s).as_str());
                            let datetime: DateTime<Utc> = DateTime::from_utc(datetime, Utc);
                            res.push(QuaintValue::datetime(datetime.into()))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::DateTime(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    ColumnType::Json => match row.remove(0) {
                        json => res.push(QuaintValue::json(json)),
                    },
                    ColumnType::Enum => match row.remove(0) {
                        serde_json::Value::String(s) => res.push(QuaintValue::enum_variant(s)),
                        serde_json::Value::Null => res.push(QuaintValue::Enum(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    ColumnType::Bytes => match row.remove(0) {
                        serde_json::Value::String(s) => res.push(QuaintValue::Bytes(Some(s.into_bytes().into()))),
                        serde_json::Value::Null => res.push(QuaintValue::Bytes(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    unimplemented => {
                        todo!("support column type: Column: {:?}", unimplemented)
                    }
                }
            }

            res
        };

        let names = js_result_set.column_names;
        let rows = js_result_set.rows.iter_mut().map(to_quaint_row).collect();

        QuaintResultSet::new(names, rows)
    }
}

impl Proxy {
    pub async fn query_raw(&self, params: Query) -> napi::Result<JSResultSet> {
        let promise = self.query_raw.call_async::<JsPromise<JSResultSet>>(params).await?;
        let value = promise.await?;
        Ok(value)
    }

    pub async fn execute_raw(&self, params: Query) -> napi::Result<u32> {
        let promise = self.execute_raw.call_async::<JsPromise<u32>>(params).await?;
        let value = promise.await?;
        Ok(value)
    }

    pub async fn version(&self) -> napi::Result<Option<String>> {
        let version = self.version.call_async::<Option<String>>(()).await?;
        Ok(version)
    }

    pub async fn close(&self) -> napi::Result<()> {
        self.close.call_async::<()>(()).await
    }

    pub fn is_healthy(&self) -> napi::Result<bool> {
        // TODO: call `is_healthy` in a blocking fashion, returning its result as a boolean.
        unimplemented!();
    }
}
