use datamodel_connector::ConnectorCapabilities;
use engine::{ConstructorOptions, QueryEngine};
use error::ApiError;
use napi::{
    threadsafe_function::ThreadSafeCallContext, CallContext, Env, JsFunction, JsObject, JsUndefined, JsUnknown,
    Property,
};
use napi_derive::{js_function, module_exports};
use prisma_models::DatamodelConverter;
use query_core::{exec_loader, schema_builder, BuildMode, QueryExecutor, QuerySchemaRef};
use request_handlers::dmmf;
use std::{collections::BTreeMap, sync::Arc};

mod engine;
mod error;
mod logger;

pub(crate) type Result<T> = std::result::Result<T, error::ApiError>;
pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

#[js_function(2)]
fn constructor(ctx: CallContext) -> napi::Result<JsUndefined> {
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

#[js_function(2)]
fn query(ctx: CallContext) -> napi::Result<JsObject> {
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
fn version(ctx: CallContext) -> napi::Result<JsUnknown> {
    #[derive(serde::Serialize, Clone, Copy)]
    struct Version {
        commit: &'static str,
        version: &'static str,
    }

    let version = Version {
        commit: env!("GIT_HASH"),
        version: env!("CARGO_PKG_VERSION"),
    };

    ctx.env.to_js_value(&version)
}

#[js_function(1)]
fn dmmf(ctx: CallContext) -> napi::Result<JsUnknown> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GetConfigOptions {
        datamodel: String,
        #[serde(default)]
        enable_raw_queries: bool,
    }

    let options = ctx.get::<JsUnknown>(0)?;
    let options: GetConfigOptions = ctx.env.from_js_value(options)?;

    let GetConfigOptions {
        datamodel,
        enable_raw_queries,
    } = options;

    let template = DatamodelConverter::convert_string(datamodel.clone());
    let config =
        datamodel::parse_configuration(&datamodel).map_err(|errors| ApiError::conversion(errors, &datamodel))?;

    let capabilities = match config.subject.datasources.first() {
        Some(datasource) => datasource.capabilities(),
        None => ConnectorCapabilities::empty(),
    };

    let source = config
        .subject
        .datasources
        .first()
        .ok_or_else(|| ApiError::configuration("No valid data source found"))?;

    let url = source
        .load_url()
        .map_err(|err| ApiError::Conversion(err, datamodel.clone()))?;

    let db_name = exec_loader::db_name(source, &url).map_err(ApiError::from)?;

    let internal_data_model = template.build(db_name);
    let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
        internal_data_model,
        BuildMode::Modern,
        enable_raw_queries,
        capabilities,
        config.subject.preview_features().cloned().collect(),
    ));

    let dm = datamodel::parse_datamodel(datamodel.as_str()).unwrap();
    let dmmf = dmmf::render_dmmf(&dm.subject, query_schema);
    let serialized = serde_json::to_string_pretty(&dmmf)?;

    ctx.env.to_js_value(&serialized)
}

#[js_function(1)]
fn get_config(ctx: CallContext) -> napi::Result<JsUnknown> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GetConfigOptions {
        datamodel: String,
        #[serde(default)]
        ignore_env_var_errors: bool,
        #[serde(default)]
        datasource_overrides: BTreeMap<String, String>,
    }

    let options = ctx.get::<JsUnknown>(0)?;
    let options: GetConfigOptions = ctx.env.from_js_value(options)?;

    let GetConfigOptions {
        datamodel,
        ignore_env_var_errors,
        datasource_overrides,
    } = options;

    let overrides: Vec<(_, _)> = datasource_overrides.into_iter().collect();
    let mut config = datamodel::parse_configuration_with_url_overrides(&datamodel, overrides)
        .map_err(|errors| ApiError::conversion(errors, &datamodel))?;

    config.subject = config
        .subject
        .validate_that_one_datasource_is_provided()
        .map_err(|errors| ApiError::conversion(errors, &datamodel))?;

    if !ignore_env_var_errors {
        config
            .subject
            .resolve_datasource_urls_from_env()
            .map_err(|errors| ApiError::conversion(errors, &datamodel))?;
    }

    let serialized = datamodel::json::mcf::config_to_mcf_json_value(&config);
    ctx.env.to_js_value(&serialized)
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
        ],
    )?;

    exports.set_named_property("QueryEngine", query_engine)?;
    exports.create_named_method("version", version)?;
    exports.create_named_method("getConfig", get_config)?;
    exports.create_named_method("dmmf", dmmf)?;

    Ok(())
}
