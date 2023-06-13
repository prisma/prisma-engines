use async_trait::async_trait;
use js_drivers::{Driver, Result, ResultSet};
use napi::bindgen_prelude::Promise as JsPromise;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};
use napi::JsObject;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

// Note: Every ThreadsafeFunction<T, ?> should have an explicit `ErrorStrategy::Fatal` set, as to avoid
// "TypeError [ERR_INVALID_ARG_TYPE]: The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. Received null".
// See: https://github.com/napi-rs/napi-rs/issues/1521.
#[derive(Clone)]
pub struct NodejsDriver {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: ThreadsafeFunction<String, ErrorStrategy::Fatal>,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: ThreadsafeFunction<String, ErrorStrategy::Fatal>,

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

#[async_trait]
impl Driver for NodejsDriver {
    async fn query_raw(&self, sql: String) -> Result<ResultSet> {
        println!("[rs] calling query_raw: {}", &sql);

        let promise = self.query_raw.call_async::<JsPromise<NodejsResultSet>>(sql).await?;

        println!("[rs] awaiting promise");
        let value = promise.await?;

        println!("[rs] awaited: {:?}", &value);
        Ok(value.into())
    }

    async fn execute_raw(&self, sql: String) -> Result<u32> {
        println!("[rs] calling execute_raw: {}", &sql);
        let promise = self.execute_raw.call_async::<JsPromise<u32>>(sql).await?;

        println!("[rs] awaiting promise");
        let value = promise.await?;
        println!("[rs] got awaited value: {:?}", &value);
        Ok(value)
    }

    async fn version(&self) -> Result<Option<String>> {
        println!("[rs] calling version");

        let version = self.version.call_async::<Option<String>>(()).await?;
        println!("[rs] version: {:?}", &version);

        Ok(version)
    }

    async fn close(&self) -> Result<()> {
        println!("[rs] calling close");
        self.close
            .call_async::<()>(())
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
    }

    fn is_healthy(&self) -> Result<bool> {
        println!("[rs] calling is_healthy");

        // TODO: call `is_healthy` in a blocking fashion, returning its result as a boolean.
        unimplemented!();
    }
}

impl NodejsDriver {
    // Reify creates a rust representation of the JS driver
    pub fn reify(js_driver: JsObject) -> napi::Result<Self> {
        let query_raw = js_driver.get_named_property("queryRaw")?;
        let execute_raw = js_driver.get_named_property("executeRaw")?;
        let version = js_driver.get_named_property("version")?;
        let close = js_driver.get_named_property("close")?;
        let is_healthy = js_driver.get_named_property("isHealthy")?;

        let driver = NodejsDriver {
            query_raw,
            execute_raw,
            version,
            close,
            is_healthy,
        };

        Ok(driver)
    }
}

#[napi(object)]
#[derive(Debug, Serialize, Deserialize)]
pub struct NodejsResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl From<NodejsResultSet> for ResultSet {
    fn from(value: NodejsResultSet) -> Self {
        ResultSet {
            columns: value.columns,
            rows: value.rows,
        }
    }
}
