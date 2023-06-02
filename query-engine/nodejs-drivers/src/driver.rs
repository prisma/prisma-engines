use napi::bindgen_prelude::Promise as JsPromise;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};
use napi::JsObject;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

// Note: Every ThreadsafeFunction<T, ?> should have an explicit `ErrorStrategy::Fatal` set, as to avoid
// "TypeError [ERR_INVALID_ARG_TYPE]: The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. Received null".
// See: https://github.com/napi-rs/napi-rs/issues/1521.
#[derive(Clone)]
pub struct Driver {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: ThreadsafeFunction<String, ErrorStrategy::Fatal>,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: ThreadsafeFunction<String, ErrorStrategy::Fatal>,

    /// Return the version of the underlying database, queried directly from the
    /// source.
    version: ThreadsafeFunction<(), ErrorStrategy::Fatal>,

    /// Return true iff the underlying database connection is healthy.
    #[allow(dead_code)]
    is_healthy: ThreadsafeFunction<(), ErrorStrategy::Fatal>,
}

// Reify creates a rust representation of the JS driver
pub fn reify(js_driver: JsObject) -> napi::Result<Driver> {
    let query_raw = js_driver.get_named_property("queryRaw")?;
    let execute_raw = js_driver.get_named_property("executeRaw")?;
    let version = js_driver.get_named_property("version")?;
    let is_healthy = js_driver.get_named_property("isHealthy")?;

    let driver = Driver {
        query_raw,
        execute_raw,
        version,
        is_healthy,
    };
    Ok(driver)
}

#[napi(object)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<String>,

    // TODO: support any JS-serializable type, not just String.
    pub rows: Vec<Vec<String>>,
}

impl Driver {
    pub async fn query_raw(&self, sql: String) -> napi::Result<ResultSet> {
        println!("[rs] calling query_raw: {}", &sql);

        let promise = self.query_raw.call_async::<JsPromise<ResultSet>>(sql).await?;

        println!("[rs] awaiting promise");
        let value = promise.await?;

        println!("[rs] awaited: {:?}", &value);
        Ok(value)
    }

    pub async fn execute_raw(&self, sql: String) -> napi::Result<u32> {
        println!("[rs] calling execute_raw: {}", &sql);
        let promise = self.execute_raw.call_async::<JsPromise<u32>>(sql).await?;

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

    pub fn is_healthy(&self) -> napi::Result<bool> {
        println!("[rs] calling is_healthy");

        // TODO: call `is_healthy` in a blocking fashion, returning its result as a boolean.
        unimplemented!();
    }
}
