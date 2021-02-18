use engine::{ConstructorOptions, QueryEngine};
use napi::{CallContext, Env, JsObject, JsUndefined, JsUnknown, Property};
use napi_derive::{js_function, module_exports};
use query_core::QueryExecutor;

mod engine;
mod error;
mod logger;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

#[js_function(1)]
fn constructor(ctx: CallContext) -> napi::Result<JsUndefined> {
    let arg0 = ctx.get::<JsUnknown>(0)?;
    let params: ConstructorOptions = ctx.env.from_js_value(arg0)?;

    let mut this: JsObject = ctx.this_unchecked();
    let engine = QueryEngine::new(params)?;

    ctx.env.wrap(&mut this, engine)?;
    ctx.env.get_undefined()
}

#[js_function(0)]
fn connect(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.connect().await?) }, |&mut env, ()| {
            env.get_undefined()
        })
}

#[js_function(0)]
fn disconnect(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.disconnect().await?) }, |&mut env, ()| {
            env.get_undefined()
        })
}

#[js_function(1)]
fn query(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    let query = ctx.get::<JsObject>(0)?;
    let body = ctx.env.from_js_value(query)?;

    ctx.env
        .execute_tokio_future(async move { Ok(engine.query(body).await?) }, |&mut env, response| {
            env.create_string(&serde_json::to_string(&response).unwrap())
        })
}

#[js_function(0)]
fn sdl_schema(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.sdl_schema().await?) }, |&mut env, schema| {
            env.create_string(&serde_json::to_string(&schema).unwrap())
        })
}

#[js_function(0)]
fn dmmf(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.dmmf().await?) }, |&mut env, dmmf| {
            env.create_string(&serde_json::to_string(&dmmf).unwrap())
        })
}

#[js_function(0)]
fn get_config(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env
        .execute_tokio_future(async move { Ok(engine.get_config().await?) }, |&mut env, config| {
            env.create_string(&serde_json::to_string(&config).unwrap())
        })
}

#[js_function(0)]
fn server_info(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env.execute_tokio_future(
        async move { Ok(engine.server_info().await?) },
        |&mut env, server_info| env.create_string(&serde_json::to_string(&server_info).unwrap()),
    )
}

#[js_function(0)]
fn next_log_event(ctx: CallContext) -> napi::Result<JsObject> {
    let this: JsObject = ctx.this_unchecked();
    let engine: &QueryEngine = ctx.env.unwrap(&this)?;
    let engine: QueryEngine = engine.clone();

    ctx.env.execute_tokio_future(
        async move { Ok(engine.next_log_event().await?) },
        |&mut env, event| match event {
            Some(event) => env.create_string(&event),
            None => env.get_null().and_then(|nil| nil.coerce_to_string()),
        },
    )
}

#[module_exports]
pub fn init(mut exports: JsObject, env: Env) -> napi::Result<()> {
    let query_engine = env.define_class(
        "QueryEngine",
        constructor,
        &[
            Property::new(&env, "connect")?.with_method(connect),
            Property::new(&env, "disconnect")?.with_method(disconnect),
            Property::new(&env, "query")?.with_method(query),
            Property::new(&env, "sdlSchema")?.with_method(sdl_schema),
            Property::new(&env, "dmmf")?.with_method(dmmf),
            Property::new(&env, "getConfig")?.with_method(get_config),
            Property::new(&env, "serverInfo")?.with_method(server_info),
            Property::new(&env, "nextLogEvent")?.with_method(next_log_event),
        ],
    )?;

    exports.set_named_property("QueryEngine", query_engine)?;

    Ok(())
}
