use napi::{threadsafe_function::ThreadSafeCallContext, CallContext, JsFunction, JsObject, JsUndefined, JsUnknown};
use napi_derive::js_function;

use crate::engine::{ConstructorOptions, QueryEngine};

#[js_function(2)]
pub fn constructor(ctx: CallContext) -> napi::Result<JsUndefined> {
    let options = ctx.get::<JsUnknown>(0)?;
    let callback = ctx.get::<JsFunction>(1)?;

    let params: ConstructorOptions = ctx.env.from_js_value(options)?;

    let mut log_callback = ctx
        .env
        .create_threadsafe_function(&callback, 0, |ctx: ThreadSafeCallContext<String>| {
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
        .execute_tokio_future(async move { Ok(engine.connect().await?) }, |&mut env, ()| {
            env.get_undefined()
        })
}

#[js_function(0)]
pub fn disconnect(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.disconnect().await?) }, |&mut env, ()| {
            env.get_undefined()
        })
}

#[js_function(2)]
pub fn query(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    let query = ctx.get::<JsObject>(0)?;
    let trace = ctx.get::<JsObject>(1)?;

    let body = ctx.env.from_js_value(query)?;
    let trace = ctx.env.from_js_value(trace)?;

    ctx.env.execute_tokio_future(
        async move { Ok(engine.query(body, trace).await?) },
        |&mut env, response| env.create_string(&serde_json::to_string(&response).unwrap()),
    )
}

#[js_function(0)]
pub fn sdl_schema(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.sdl_schema().await?) }, |&mut env, schema| {
            env.create_string(&serde_json::to_string(&schema).unwrap())
        })
}
