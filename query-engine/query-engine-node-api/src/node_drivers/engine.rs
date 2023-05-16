use crate::engine::QueryEngine;

use napi::bindgen_prelude::Promise as JsPromise;
use napi::{
    threadsafe_function::{ErrorStrategy, ThreadsafeFunction},
    Env, JsFunction, JsObject, JsUnknown,
};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[napi(object)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

// Note: Every ThreadsafeFunction<T, ?> should have an explicit `ErrorStrategy::Fatal` set, as to avoid
// "TypeError [ERR_INVALID_ARG_TYPE]: The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. Received null".
// See: https://github.com/napi-rs/napi-rs/issues/1521.
pub(crate) struct NodejsFunctionContext {
    // TODO: maybe env is no longer needed
    _env: Env,

    /// Execute a query given as SQL, interpolating the given parameters.
    pub query_raw: ThreadsafeFunction<String, ErrorStrategy::Fatal>,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    pub execute_raw: ThreadsafeFunction<String, ErrorStrategy::Fatal>,

    /// Return the version of the underlying database, queried directly from the
    /// source.
    pub version: ThreadsafeFunction<(), ErrorStrategy::Fatal>,
}

unsafe impl Send for NodejsFunctionContext {}
unsafe impl Sync for NodejsFunctionContext {}

impl NodejsFunctionContext {
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

    // TODO: is it possible to remove `async` here?
    pub async fn version(&self) -> napi::Result<Option<String>> {
        println!("[rs] calling version");

        let version = self.version.call_async::<Option<String>>(()).await?;
        println!("[rs] version: {:?}", &version);

        Ok(version)
    }
}

fn read_nodejs_functions(env: Env, ctx: JsObject) -> napi::Result<NodejsFunctionContext> {
    let query_raw = ctx.get_named_property("queryRaw")?;
    let execute_raw = ctx.get_named_property("executeRaw")?;
    let version = ctx.get_named_property("version")?;

    let ctx = NodejsFunctionContext {
        _env: env,
        query_raw,
        execute_raw,
        version,
    };
    Ok(ctx)
}

// Wrapper for the main query engine that keeps track of the JS-provided functions
// used to execute queries.
#[napi]
pub struct QueryEngineNodeDrivers {
    pub(crate) engine: QueryEngine,
    pub(crate) fn_ctx: NodejsFunctionContext,
}

#[napi]
impl QueryEngineNodeDrivers {
    /// Parse a validated datamodel and configuration to allow connecting later on.
    /// Wraps the `QueryEngine::new` constructor.
    #[napi(constructor)]
    pub fn new(napi_env: Env, options: JsUnknown, callback: JsFunction, fn_ctx: JsObject) -> napi::Result<Self> {
        let fn_ctx = read_nodejs_functions(napi_env, fn_ctx)?;
        let engine = QueryEngine::new(napi_env, options, callback)?;

        Ok(Self { engine, fn_ctx })
    }

    /// Note: call this function if you want to quickly test the async/await functionality.
    #[napi]
    pub async fn test_async(&self, sql: String) -> napi::Result<ResultSet> {
        let _version = self.fn_ctx.version().await?;
        let result_set = self.fn_ctx.query_raw(sql).await?;
        Ok(result_set)
    }

    /// Connect to the database, allow queries to be run.
    #[napi]
    pub async fn connect(&self, _trace: String) -> napi::Result<()> {
        self.engine.connect(_trace).await
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    #[napi]
    pub async fn disconnect(&self, _trace: String) -> napi::Result<()> {
        self.engine.disconnect(_trace).await
    }

    /// If connected, sends a query to the core and returns the response.
    #[napi]
    pub async fn query(&self, body: String, _trace: String, tx_id: Option<String>) -> napi::Result<String> {
        self.engine.query(body, _trace, tx_id).await
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    #[napi]
    pub async fn start_transaction(&self, input: String, _trace: String) -> napi::Result<String> {
        self.engine.start_transaction(input, _trace).await
    }

    /// If connected, attempts to commit a transaction with id `tx_id` in the core.
    #[napi]
    pub async fn commit_transaction(&self, tx_id: String, _trace: String) -> napi::Result<String> {
        self.engine.commit_transaction(tx_id, _trace).await
    }

    #[napi]
    pub async fn dmmf(&self, _trace: String) -> napi::Result<String> {
        self.engine.dmmf(_trace).await
    }

    #[napi]
    pub async fn rollback_transaction(&self, tx_id: String, _trace: String) -> napi::Result<String> {
        self.engine.rollback_transaction(tx_id, _trace).await
    }

    #[napi]
    pub async fn sdl_schema(&self) -> napi::Result<String> {
        self.engine.sdl_schema().await
    }

    #[napi]
    pub async fn metrics(&self, _json_options: String) -> napi::Result<String> {
        self.engine.metrics(_json_options).await
    }
}
