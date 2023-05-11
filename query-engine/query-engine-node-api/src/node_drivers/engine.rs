use crate::engine::QueryEngine;

#[allow(unused_imports)]
use napi::bindgen_prelude::Promise as JsPromise;

use napi::{Env, JsFunction, JsObject, JsString, JsUnknown};
use napi_derive::napi;

pub struct NodejsFunctionContext {
    env: Env,
    pub query_raw: JsFunction,
    pub query_raw_typed: JsFunction,
    pub execute_raw: JsFunction,
    pub execute_raw_typed: JsFunction,
    pub version: JsFunction,
}

impl NodejsFunctionContext {
    pub fn query_raw(&self) -> napi::Result<()> {
        let arg = self.env.create_string("SELECT * FROM users").unwrap();
        let _promise: JsUnknown = self.query_raw.call(None, &[arg])?;
        // TODO: cast JsUnknown to JsPromise

        Ok(())
    }

    pub fn query_raw_typed(&self) -> napi::Result<()> {
        let arg = self.env.create_string("SELECT * FROM users").unwrap();
        let _promise: JsUnknown = self.query_raw_typed.call(None, &[arg])?;
        // TODO: cast JsUnknown to JsPromise, idk how yet :/

        Ok(())
    }

    pub fn execute_raw(&self) -> napi::Result<()> {
        let arg = self.env.create_string("EXECUTE ...").unwrap();
        let _promise: JsUnknown = self.execute_raw.call(None, &[arg])?;
        // TODO: cast JsUnknown to JsPromise, idk how yet :/

        Ok(())
    }

    pub fn execute_raw_typed(&self) -> napi::Result<()> {
        let arg = self.env.create_string("EXECUTE ...").unwrap();
        let _promise: JsUnknown = self.execute_raw_typed.call(None, &[arg])?;
        // TODO: cast JsUnknown to JsPromise, idk how yet :/

        Ok(())
    }

    pub fn version(&self) -> napi::Result<String> {
        let version_raw_str: JsUnknown = self.version.call::<JsUnknown>(None, &[])?;
        let version_js_str: JsString = version_raw_str.try_into()?;
        let version_str = version_js_str.into_utf8()?.into_owned()?;

        Ok(version_str)
    }
}

fn read_nodejs_functions<'a>(env: Env, ctx: JsObject) -> napi::Result<NodejsFunctionContext> {
    let query_raw = ctx.get_named_property("queryRaw")?;
    let query_raw_typed = ctx.get_named_property("queryRawTyped")?;
    let execute_raw = ctx.get_named_property("executeRaw")?;
    let execute_raw_typed = ctx.get_named_property("executeRawTyped")?;
    let version = ctx.get_named_property("version")?;

    let ctx = NodejsFunctionContext {
        env,
        query_raw,
        query_raw_typed,
        execute_raw,
        execute_raw_typed,
        version,
    };
    Ok(ctx)
}

// Wrapper for the main query engine that keeps track of the JS-provided functions
// used to execute queries.
#[napi]
pub struct QueryEngineNodeDrivers {
    pub(crate) engine: QueryEngine,

    #[allow(dead_code)]
    pub(crate) fn_ctx: NodejsFunctionContext,
}

#[napi]
impl QueryEngineNodeDrivers {
    /// Parse a validated datamodel and configuration to allow connecting later on.
    /// Wraps the `QueryEngine::new` constructor.
    #[napi(constructor)]
    pub fn new(napi_env: Env, options: JsUnknown, callback: JsFunction, fn_ctx: JsObject) -> napi::Result<Self> {
        let fn_ctx = read_nodejs_functions(napi_env.clone(), fn_ctx)?;
        let engine = QueryEngine::new(napi_env, options, callback)?;

        Ok(Self { engine, fn_ctx })
    }

    /// Connect to the database, allow queries to be run.
    pub async fn connect(&self, _trace: String) -> napi::Result<()> {
        self.engine.connect(_trace).await
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    pub async fn disconnect(&self, _trace: String) -> napi::Result<()> {
        self.engine.disconnect(_trace).await
    }

    /// If connected, sends a query to the core and returns the response.
    pub async fn query(&self, body: String, _trace: String, tx_id: Option<String>) -> napi::Result<String> {
        self.engine.query(body, _trace, tx_id).await
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    pub async fn start_transaction(&self, input: String, _trace: String) -> napi::Result<String> {
        self.engine.start_transaction(input, _trace).await
    }

    /// If connected, attempts to commit a transaction with id `tx_id` in the core.
    pub async fn commit_transaction(&self, tx_id: String, _trace: String) -> napi::Result<String> {
        self.engine.commit_transaction(tx_id, _trace).await
    }

    pub async fn dmmf(&self, _trace: String) -> napi::Result<String> {
        self.engine.dmmf(_trace).await
    }

    pub async fn rollback_transaction(&self, tx_id: String, _trace: String) -> napi::Result<String> {
        self.engine.rollback_transaction(tx_id, _trace).await
    }

    pub async fn sdl_schema(&self) -> napi::Result<String> {
        self.engine.sdl_schema().await
    }

    pub async fn metrics(&self, _json_options: String) -> napi::Result<String> {
        self.engine.metrics(_json_options).await
    }
}
