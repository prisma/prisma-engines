use core::panic;

use napi::bindgen_prelude::{FromNapiValue, Promise as JsPromise, ToNapiValue};
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};
use napi::JsObject;
use napi_derive::napi;
use quaint::connector::ResultSet as QuaintResultSet;
use quaint::Value as QuaintValue;

// Note: Every ThreadsafeFunction<T, ?> should have an explicit `ErrorStrategy::Fatal` set, as to avoid
// "TypeError [ERR_INVALID_ARG_TYPE]: The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. Received null".
// See: https://github.com/napi-rs/napi-rs/issues/1521.
#[derive(Clone)]
pub struct Driver {
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

// Reify creates a rust representation of the JS driver
pub fn reify(js_driver: JsObject) -> napi::Result<Driver> {
    let query_raw = js_driver.get_named_property("queryRaw")?;
    let execute_raw = js_driver.get_named_property("executeRaw")?;
    let version = js_driver.get_named_property("version")?;
    let close = js_driver.get_named_property("close")?;
    let is_healthy = js_driver.get_named_property("isHealthy")?;

    let driver = Driver {
        query_raw,
        execute_raw,
        version,
        close,
        is_healthy,
    };
    Ok(driver)
}

// This result set is more convenient to be manipulated from both Rust and NodeJS.
// Quaint's version of  ResultSet is:
//
// pub struct ResultSet {
//     pub(crate) columns: Arc<Vec<String>>,
//     pub(crate) rows: Vec<Vec<Value<'static>>>,
//     pub(crate) last_insert_id: Option<u64>,
// }
//
// If we used this ResultSet would we would have worse ergonomics as quaint::Value is a structured
// enum and cannot be used directly with the #[napi(Object)] macro. Thus requiring us to implement
// the FromNapiValue and ToNapiValue traits for quaint::Value, and use a different custom type
// representing the Value in javascript.
//
#[napi(object)]
#[derive(Debug)]
pub struct ResultSet {
    pub column_types: Vec<ColumnType>,
    pub column_names: Vec<String>,
    // Note this might be encoded differently for performance reasons
    pub rows: Vec<Vec<serde_json::Value>>,
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

impl From<ResultSet> for QuaintResultSet {
    fn from(mut val: ResultSet) -> Self {
        // TODO: extract, todo: error rather than panic?
        let to_quaint_row = move |row: &mut Vec<serde_json::Value>| -> Vec<quaint::Value<'static>> {
            let mut res = Vec::with_capacity(row.len());

            for i in 0..row.len() {
                match val.column_types[i] {
                    ColumnType::Int64 => match row.remove(0) {
                        serde_json::Value::Number(n) => {
                            res.push(QuaintValue::int64(n.as_i64().expect("number must be an i64")))
                        }
                        serde_json::Value::Null => todo!(),
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

impl Driver {
    pub async fn query_raw(&self, params: Query) -> napi::Result<ResultSet> {
        println!("[rs] calling query_raw: {:?}", &params);

        let promise = self.query_raw.call_async::<JsPromise<ResultSet>>(params).await?;

        println!("[rs] awaiting promise");
        let value = promise.await?;

        println!("[rs] awaited: {:?}", &value);
        Ok(value)
    }

    pub async fn execute_raw(&self, params: Query) -> napi::Result<u32> {
        println!("[rs] calling execute_raw: {:?}", &params);
        let promise = self.execute_raw.call_async::<JsPromise<u32>>(params).await?;

        println!("[rs] awaiting promise");
        let value = promise.await?;
        println!("[rs] got awaited value: {:?}", &value);
        Ok(value)
    }

    pub async fn version(&self) -> napi::Result<Option<String>> {
        println!("[rs] calling version");

        let version = self.version.call_async::<Option<String>>(()).await?;
        println!("[rs] version: {:?}", &version);

        Ok(version)
    }

    pub async fn close(&self) -> napi::Result<()> {
        println!("[rs] calling close");
        self.close.call_async::<()>(()).await
    }

    pub fn is_healthy(&self) -> napi::Result<bool> {
        println!("[rs] calling is_healthy");

        // TODO: call `is_healthy` in a blocking fashion, returning its result as a boolean.
        unimplemented!();
    }
}
