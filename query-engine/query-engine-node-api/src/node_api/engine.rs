use napi::{JsFunction, Env, JsUnknown, threadsafe_function::ThreadSafeCallContext};
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

            ctx.env.create_string_from_std(ctx.value).map(|js_string| vec![js_string])
        })?;

        log_callback.unref(&env)?;

        Ok(Self(crate::engine::QueryEngine::new(params, log_callback)?))
    }

    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        self.0.connect().await?;
        Ok(())
    }

    #[napi]
    pub async fn disconnect(&self) -> napi::Result<()> {
        self.0.disconnect().await?;
        Ok(())
    }

    #[napi]
    pub async fn query(&self, body: String, trace: String, tx_id: Option<String>) -> napi::Result<String> {
        let body = serde_json::from_str(&body)?;
        let trace = serde_json::from_str(&trace)?;

        let response = self.0.query(body, trace, tx_id).await?;

        Ok(serde_json::to_string(&response)?)
    }

    #[napi]
    pub async fn sdl_schema(&self) -> napi::Result<String> {
        Ok(self.0.sdl_schema().await?)
    }

    #[napi]
    pub async fn start_transaction(&self, input: String, trace: String) -> napi::Result<String> {
        let input = serde_json::from_str(&input)?;
        let trace = serde_json::from_str(&trace)?;

        Ok(self.0.start_tx(input, trace).await?)
    }

    #[napi]
    pub async fn commit_transaction(&self, tx_id: String, trace: String) -> napi::Result<String> {
        let trace = serde_json::from_str(&trace)?;
        Ok(self.0.commit_tx(tx_id, trace).await?)
    }

    #[napi]
    pub async fn rollback_transaction(&self, tx_id: String, trace: String) -> napi::Result<String> {
        let trace = serde_json::from_str(&trace)?;
        Ok(self.0.rollback_tx(tx_id, trace).await?)
    }
}

// #[js_function(0)]
// pub fn connect(ctx: CallContext) -> napi::Result<JsObject> {
//     let this: JsObject = ctx.this_unchecked();
//     let engine: &QueryEngine = ctx.env.unwrap(&this)?;
//     let engine: QueryEngine = engine.clone();

//     ctx.env
//         .execute_tokio_future(async move { Ok(engine.connect().await?) }, |env, ()| {
//             env.get_undefined()
//         })
// }

// #[js_function(0)]
// pub fn disconnect(ctx: CallContext) -> napi::Result<JsObject> {
//     let this: JsObject = ctx.this_unchecked();
//     let engine: &QueryEngine = ctx.env.unwrap(&this)?;
//     let engine: QueryEngine = engine.clone();

//     ctx.env
//         .execute_tokio_future(async move { Ok(engine.disconnect().await?) }, |env, ()| {
//             env.get_undefined()
//         })
// }

// #[js_function(3)]
// pub fn query(ctx: CallContext) -> napi::Result<JsObject> {
//     let this: JsObject = ctx.this_unchecked();
//     let engine: &QueryEngine = ctx.env.unwrap(&this)?;
//     let engine: QueryEngine = engine.clone();

//     let body = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;
//     let body = serde_json::from_str(&body)?;

//     let trace = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;
//     let trace = serde_json::from_str(&trace)?;

//     let tx_id: Option<JsString> = match ctx.try_get::<JsString>(2) {
//         Ok(either) => either.into(),
//         Err(_) => None,
//     };

//     let tx_id = match tx_id {
//         Some(js_string) => Some(js_string.into_utf8()?.into_owned()?),
//         _ => None,
//     };

//     ctx.env.execute_tokio_future(
//         async move { Ok(engine.query(body, trace, tx_id).await?) },
//         |env, response| {
//             let res = serde_json::to_string(&response).unwrap();

//             env.adjust_external_memory(res.len() as i64)?;
//             env.create_string_from_std(res)
//         },
//     )
// }

// #[js_function(0)]
// pub fn sdl_schema(ctx: CallContext) -> napi::Result<JsObject> {
//     let this: JsObject = ctx.this_unchecked();
//     let engine: &QueryEngine = ctx.env.unwrap(&this)?;
//     let engine: QueryEngine = engine.clone();

//     ctx.env
//         .execute_tokio_future(async move { Ok(engine.sdl_schema().await?) }, |env, schema| {
//             let res = serde_json::to_string(&schema).unwrap();
//             env.adjust_external_memory(res.len() as i64)?;
//             env.create_string_from_std(res)
//         })
// }

// #[js_function(2)]
// pub fn start_transaction(ctx: CallContext) -> napi::Result<JsObject> {
//     let this: JsObject = ctx.this_unchecked();
//     let engine: &QueryEngine = ctx.env.unwrap(&this)?;
//     let engine: QueryEngine = engine.clone();

//     let input = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;
//     let input = serde_json::from_str(&input)?;

//     let trace = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;
//     let trace = serde_json::from_str(&trace)?;

//     ctx.env
//         .execute_tokio_future(async move { Ok(engine.start_tx(input, trace).await?) }, |env, data| {
//             env.adjust_external_memory(data.len() as i64)?;
//             env.create_string_from_std(data)
//         })
// }

// #[js_function(2)]
// pub fn commit_transaction(ctx: CallContext) -> napi::Result<JsObject> {
//     let this: JsObject = ctx.this_unchecked();
//     let engine: &QueryEngine = ctx.env.unwrap(&this)?;
//     let engine: QueryEngine = engine.clone();

//     let tx_id = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;

//     let trace = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;
//     let trace = serde_json::from_str(&trace)?;

//     ctx.env
//         .execute_tokio_future(async move { Ok(engine.commit_tx(tx_id, trace).await?) }, |env, data| {
//             env.adjust_external_memory(data.len() as i64)?;
//             env.create_string_from_std(data)
//         })
// }

// #[js_function(2)]
// pub fn rollback_transaction(ctx: CallContext) -> napi::Result<JsObject> {
//     let this: JsObject = ctx.this_unchecked();
//     let engine: &QueryEngine = ctx.env.unwrap(&this)?;
//     let engine: QueryEngine = engine.clone();

//     let tx_id = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;

//     let trace = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;
//     let trace = serde_json::from_str(&trace)?;

//     ctx.env.execute_tokio_future(
//         async move { Ok(engine.rollback_tx(tx_id, trace).await?) },
//         |env, data| {
//             env.adjust_external_memory(data.len() as i64)?;
//             env.create_string_from_std(data)
//         },
//     )
// }
