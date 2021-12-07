use napi::{threadsafe_function::ThreadSafeCallContext, Env, JsFunction, JsUnknown};
use napi_derive::napi;

use crate::engine::ConstructorOptions;

#[napi]
pub struct QueryEngine(crate::engine::QueryEngine);

#[napi]
impl QueryEngine {
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

        Ok(Self(crate::engine::QueryEngine::new(params, log_callback)?))
    }

    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        let engine = self.0.clone();
        engine.connect().await?;
        Ok(())
    }

    #[napi]
    pub async fn disconnect(&self) -> napi::Result<()> {
        let engine = self.0.clone();
        engine.disconnect().await?;
        Ok(())
    }

    #[napi]
    pub async fn query(&self, body: String, trace: String, tx_id: Option<String>) -> napi::Result<String> {
        let engine: crate::engine::QueryEngine = self.0.clone();
        let body = serde_json::from_str(&body)?;
        let trace = serde_json::from_str(&trace)?;
        let response = engine.query(body, trace, tx_id).await?;
        let res = serde_json::to_string(&response)?;
        Ok(res)
    }

    #[napi]
    pub async fn sdl_schema(&self) -> napi::Result<String> {
        let engine: crate::engine::QueryEngine = self.0.clone();
        Ok(engine.sdl_schema().await?)
    }

    #[napi]
    pub async fn start_transaction(&self, input: String, tx_id: String) -> napi::Result<String> {
        let engine: crate::engine::QueryEngine = self.0.clone();
        let input = serde_json::from_str(&input)?;
        let trace = serde_json::from_str(&tx_id)?;
        Ok(engine.start_tx(input, trace).await?)
    }

    #[napi]
    pub async fn commit_transaction(&self, tx_id: String, trace: String) -> napi::Result<String> {
        let engine: crate::engine::QueryEngine = self.0.clone();

        let trace = serde_json::from_str(&trace)?;
        Ok(engine.commit_tx(tx_id, trace).await?)
    }

    #[napi]
    pub async fn rollback_transaction(&self, tx_id: String, trace: String) -> napi::Result<String> {
        let engine: crate::engine::QueryEngine = self.0.clone();
        let trace = serde_json::from_str(&trace)?;
        Ok(engine.rollback_tx(tx_id, trace).await?)
    }
}
