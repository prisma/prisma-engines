use napi::{threadsafe_function::ThreadSafeCallContext, Env, JsFunction, JsUnknown};
use napi_derive::napi;

use crate::engine::ConstructorOptions;

/// The Node API interface layer of the Query Engine.
#[napi]
pub struct QueryEngine {
    inner: crate::engine::QueryEngine,
}

#[napi]
impl QueryEngine {
    /// Creates a new Query Engine, doesn't connect.
    #[napi(constructor)]
    pub fn new(env: Env, options: JsUnknown, callback: JsFunction) -> napi::Result<Self> {
        let params: ConstructorOptions = env.from_js_value(options)?;
        let mut log_callback = callback.create_threadsafe_function(0, |mut ctx: ThreadSafeCallContext<String>| {
            ctx.env.adjust_external_memory(ctx.value.len() as i64)?;

            ctx.env
                .create_string_from_std(ctx.value)
                .map(|js_string| vec![js_string])
        })?;

        log_callback.unref(&env)?;

        Ok(Self {
            inner: crate::engine::QueryEngine::new(params, log_callback)?,
        })
    }

    /// Connect to the underlying database.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        self.inner.connect().await?;
        Ok(())
    }

    /// Disconnect from the underlying database.
    #[napi]
    pub async fn disconnect(&self) -> napi::Result<()> {
        self.inner.disconnect().await?;
        Ok(())
    }

    /// Perform a GraphQL query.
    #[napi]
    pub async fn query(&self, body: String, trace: String, tx_id: Option<String>) -> napi::Result<String> {
        let body = serde_json::from_str(&body)?;
        let trace = serde_json::from_str(&trace)?;
        let response = self.inner.query(body, trace, tx_id).await?;
        let res = serde_json::to_string(&response)?;
        Ok(res)
    }

    /// Return the underlying schema.
    #[napi]
    pub async fn sdl_schema(&self) -> napi::Result<String> {
        Ok(self.inner.sdl_schema().await?)
    }

    /// Start a long-running transaction.
    #[napi]
    pub async fn start_transaction(&self, input: String, tx_id: String) -> napi::Result<String> {
        let input = serde_json::from_str(&input)?;
        let trace = serde_json::from_str(&tx_id)?;
        Ok(self.inner.start_tx(input, trace).await?)
    }

    /// Commit a long-running transaction.
    #[napi]
    pub async fn commit_transaction(&self, tx_id: String, trace: String) -> napi::Result<String> {
        let trace = serde_json::from_str(&trace)?;
        Ok(self.inner.commit_tx(tx_id, trace).await?)
    }

    /// Rollback a long-running transaction.
    #[napi]
    pub async fn rollback_transaction(&self, tx_id: String, trace: String) -> napi::Result<String> {
        let trace = serde_json::from_str(&trace)?;
        Ok(self.inner.rollback_tx(tx_id, trace).await?)
    }
}
