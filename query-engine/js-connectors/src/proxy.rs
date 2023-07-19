use core::panic;

use napi::bindgen_prelude::{FromNapiValue, Promise as JsPromise, ToNapiValue};
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};
use napi::JsObject;
use napi_derive::napi;
use quaint::connector::ResultSet as QuaintResultSet;
use quaint::Value as QuaintValue;

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
    Int32,
    Int64,
    Float,
    Double,
    Text,
    Enum,
    Bytes,
    Boolean,
    Char,
    Array,
    Numeric,
    Json,
    DateTime,
    Date,
    Time,
}

#[napi(object)]
#[derive(Debug)]
pub struct Query {
    pub sql: String,
    pub args: Vec<serde_json::Value>,
}

impl From<JSResultSet> for QuaintResultSet {
    fn from(mut val: JSResultSet) -> Self {
        // TODO: extract, todo: error rather than panic?
        let to_quaint_row = move |row: &mut Vec<serde_json::Value>| -> Vec<quaint::Value<'static>> {
            let mut res = Vec::with_capacity(row.len());

            for i in 0..row.len() {
                match val.column_types[i] {
                    ColumnType::Int64 => match row.remove(0) {
                        serde_json::Value::Number(n) => {
                            res.push(QuaintValue::int64(n.as_i64().expect("number must be an i64")))
                        }
                        serde_json::Value::Null => res.push(QuaintValue::Int64(None)),
                        mismatch => panic!("Expected a number, found {:?}", mismatch),
                    },
                    ColumnType::Text => match row.remove(0) {
                        serde_json::Value::String(s) => res.push(QuaintValue::text(s)),
                        serde_json::Value::Null => res.push(QuaintValue::Text(None)),
                        mismatch => panic!("Expected a string, found {:?}", mismatch),
                    },
                    unimplemented => {
                        todo!("support column type: Column: {:?}", unimplemented)
                    }
                }
            }

            res
        };

        let names = val.column_names;
        let rows = val.rows.iter_mut().map(to_quaint_row).collect();

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
