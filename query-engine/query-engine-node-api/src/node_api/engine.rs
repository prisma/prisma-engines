use napi::{
    threadsafe_function::ThreadSafeCallContext, CallContext, JsFunction, JsObject, JsString, JsUndefined, JsUnknown,
};
use napi_derive::js_function;

use crate::engine::{ConstructorOptions, QueryEngine};

#[js_function(2)]
pub fn constructor(ctx: CallContext) -> napi::Result<JsUndefined> {
    let options = ctx.get::<JsUnknown>(0)?;
    let callback = ctx.get::<JsFunction>(1)?;

    let params: ConstructorOptions = ctx.env.from_js_value(options)?;

    let mut log_callback =
        ctx.env
            .create_threadsafe_function(&callback, 0, |mut ctx: ThreadSafeCallContext<String>| {
                ctx.env.adjust_external_memory(ctx.value.len() as i64)?;

                ctx.env
                    .create_string_from_std(ctx.value)
                    .map(|js_string| vec![js_string])
            })?;

    log_callback.unref(&ctx.env)?;

    let mut this: JsObject = ctx.this_unchecked();
    let engine = QueryEngine::new(params, log_callback)?;

    ctx.env.wrap(&mut this, engine)?;
    ctx.env.get_undefined()
}

#[js_function(0)]
pub fn connect(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.connect().await?) }, |env, ()| {
            env.get_undefined()
        })
}

#[js_function(0)]
pub fn disconnect(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.disconnect().await?) }, |env, ()| {
            env.get_undefined()
        })
}

#[js_function(3)]
pub fn query(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    let body = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;
    let body = serde_json::from_str(&body)?;

    let trace = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;
    let trace = serde_json::from_str(&trace)?;

    let tx_id: Option<JsString> = match ctx.try_get::<JsString>(2) {
        Ok(either) => either.into(),
        Err(_) => None,
    };

    let tx_id = match tx_id {
        Some(js_string) => Some(js_string.into_utf8()?.into_owned()?),
        _ => None,
    };

    ctx.env.execute_tokio_future(
        async move { Ok(engine.query(body, trace, tx_id).await?) },
        |env, response| {
            let res = serde_json::to_string(&response).unwrap();

            env.adjust_external_memory(res.len() as i64)?;
            env.create_string_from_std(res)
        },
    )
}

#[js_function(0)]
pub fn sdl_schema(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.sdl_schema().await?) }, |env, schema| {
            let res = serde_json::to_string(&schema).unwrap();
            env.adjust_external_memory(res.len() as i64)?;
            env.create_string_from_std(res)
        })
}

#[js_function(2)]
pub fn start_transaction(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    let input = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;
    let input = serde_json::from_str(&input)?;

    let trace = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;
    let trace = serde_json::from_str(&trace)?;

    ctx.env
        .execute_tokio_future(async move { Ok(engine.start_tx(input, trace).await?) }, |env, data| {
            env.adjust_external_memory(data.len() as i64)?;
            env.create_string_from_std(data)
        })
}

#[js_function(2)]
pub fn commit_transaction(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    let tx_id = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;

    let trace = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;
    let trace = serde_json::from_str(&trace)?;

    ctx.env
        .execute_tokio_future(async move { Ok(engine.commit_tx(tx_id, trace).await?) }, |env, data| {
            env.adjust_external_memory(data.len() as i64)?;
            env.create_string_from_std(data)
        })
}

#[js_function(2)]
pub fn rollback_transaction(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    let tx_id = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;

    let trace = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;
    let trace = serde_json::from_str(&trace)?;

    ctx.env.execute_tokio_future(
        async move { Ok(engine.rollback_tx(tx_id, trace).await?) },
        |env, data| {
            env.adjust_external_memory(data.len() as i64)?;
            env.create_string_from_std(data)
        },
    )
}
