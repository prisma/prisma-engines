#![allow(dead_code)]
#![allow(unused_variables)]

use crate::{
    error::ApiError,
    logger::{LogCallback, Logger},
};
use js_sys::{Function as JsFunction, Object as JsObject};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing_subscriber::filter::LevelFilter;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

/// The main query engine used by JS
#[wasm_bindgen]
pub struct QueryEngine {
    inner: RwLock<Inner>,
    logger: Logger,
}

/// The state of the engine.
enum Inner {
    /// Not connected, holding all data to form a connection.
    Builder(EngineBuilder),
    /// A connected engine, holding all data to disconnect and form a new
    /// connection. Allows querying when on this state.
    Connected(ConnectedEngine),
}

/// Everything needed to connect to the database and have the core running.
struct EngineBuilder {
    schema: Arc<psl::ValidatedSchema>,
    config_dir: PathBuf,
    env: HashMap<String, String>,
}

/// Internal structure for querying and reconnecting with the engine.
struct ConnectedEngine {
    schema: Arc<psl::ValidatedSchema>,
    config_dir: PathBuf,
    env: HashMap<String, String>,
}

/// Returned from the `serverInfo` method in javascript.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    commit: String,
    version: String,
    primary_connector: Option<String>,
}

/// Parameters defining the construction of an engine.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConstructorOptions {
    datamodel: String,
    log_level: String,
    #[serde(default)]
    log_queries: bool,
    #[serde(default)]
    datasource_overrides: BTreeMap<String, String>,
    #[serde(default)]
    env: serde_json::Value,
    config_dir: PathBuf,
    #[serde(default)]
    ignore_env_var_errors: bool,
}

impl Inner {
    /// Returns a builder if the engine is not connected
    fn as_builder(&self) -> crate::Result<&EngineBuilder> {
        match self {
            Inner::Builder(ref builder) => Ok(builder),
            Inner::Connected(_) => Err(ApiError::AlreadyConnected),
        }
    }

    /// Returns the engine if connected
    fn as_engine(&self) -> crate::Result<&ConnectedEngine> {
        match self {
            Inner::Builder(_) => Err(ApiError::NotConnected),
            Inner::Connected(ref engine) => Ok(engine),
        }
    }
}

#[wasm_bindgen]
impl QueryEngine {
    /// Parse a validated datamodel and configuration to allow connecting later on.
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: JsValue,
        callback: JsFunction,
        maybe_adapter: Option<JsObject>,
    ) -> Result<QueryEngine, wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::new()`");

        let log_callback = LogCallback(callback);
        log::info!("Parsed `log_callback`");

        let ConstructorOptions {
            datamodel,
            log_level,
            log_queries,
            datasource_overrides,
            env,
            config_dir,
            ignore_env_var_errors,
        } = serde_wasm_bindgen::from_value(options).expect(
            r###"
            Failed to deserialize constructor options. 
            
            This usually happens when the javascript object passed to the constructor is missing 
            properties for the ConstructorOptions fields that must have some value.
            
            If you set some of these in javascript trough environment variables, make sure there are
            values for data_model, log_level, and any field that is not Option<T>
            "###,
        );
        log::info!("Parsed `ConstructorOptions`");

        let env = stringify_env_values(env)?; // we cannot trust anything JS sends us from process.env
        let overrides: Vec<(_, _)> = datasource_overrides.into_iter().collect();

        let mut schema = psl::validate(datamodel.into());
        let config = &mut schema.configuration;

        schema
            .diagnostics
            .to_result()
            .map_err(|err| ApiError::conversion(err, schema.db.source()))?;

        config
            .resolve_datasource_urls_query_engine(
                &overrides,
                |key| env.get(key).map(ToString::to_string),
                ignore_env_var_errors,
            )
            .map_err(|err| ApiError::conversion(err, schema.db.source()))?;

        config
            .validate_that_one_datasource_is_provided()
            .map_err(|errors| ApiError::conversion(errors, schema.db.source()))?;

        let builder = EngineBuilder {
            schema: Arc::new(schema),
            config_dir,
            env,
        };

        let log_level = log_level.parse::<LevelFilter>().unwrap();
        let logger = Logger::new(log_queries, log_level, log_callback);

        Ok(Self {
            inner: RwLock::new(Inner::Builder(builder)),
            logger,
        })
    }

    /// Connect to the database, allow queries to be run.
    #[wasm_bindgen]
    pub async fn connect(&self, trace: String) -> Result<(), wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::connect()`");
        Ok(())
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    #[wasm_bindgen]
    pub async fn disconnect(&self, trace: String) -> Result<(), wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::disconnect()`");
        Ok(())
    }

    /// If connected, sends a query to the core and returns the response.
    #[wasm_bindgen]
    pub async fn query(
        &self,
        body: String,
        trace: String,
        tx_id: Option<String>,
    ) -> Result<String, wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::query()`");
        Err(ApiError::configuration("Can't use `query` until `request_handlers` is Wasm-compatible.").into())
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    #[wasm_bindgen(js_name = startTransaction)]
    pub async fn start_transaction(&self, input: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::start_transaction()`");
        Err(ApiError::configuration("Can't use `start_transaction` until `query_core` is Wasm-compatible.").into())
    }

    /// If connected, attempts to commit a transaction with id `tx_id` in the core.
    #[wasm_bindgen(js_name = commitTransaction)]
    pub async fn commit_transaction(&self, tx_id: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::commit_transaction()`");
        Err(ApiError::configuration("Can't use `commit_transaction` until `query_core` is Wasm-compatible.").into())
    }

    #[wasm_bindgen]
    pub async fn dmmf(&self, trace: String) -> Result<String, wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::dmmf()`");
        Err(ApiError::configuration("Can't use `dmmf` until `request_handlers` is Wasm-compatible.").into())
    }

    /// If connected, attempts to roll back a transaction with id `tx_id` in the core.
    #[wasm_bindgen(js_name = rollbackTransaction)]
    pub async fn rollback_transaction(&self, tx_id: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::rollback_transaction()`");
        Ok("{}".to_owned())
    }

    /// Loads the query schema. Only available when connected.
    #[wasm_bindgen(js_name = sdlSchema)]
    pub async fn sdl_schema(&self) -> Result<String, wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::sdl_schema()`");
        Ok("{}".to_owned())
    }

    #[wasm_bindgen]
    pub async fn metrics(&self, json_options: String) -> Result<(), wasm_bindgen::JsError> {
        log::info!("Called `QueryEngine::metrics()`");
        Err(ApiError::configuration("Metrics is not enabled in Wasm.").into())
    }
}

fn stringify_env_values(origin: serde_json::Value) -> crate::Result<HashMap<String, String>> {
    use serde_json::Value;

    let msg = match origin {
        Value::Object(map) => {
            let mut result: HashMap<String, String> = HashMap::new();

            for (key, val) in map.into_iter() {
                match val {
                    Value::Null => continue,
                    Value::String(val) => {
                        result.insert(key, val);
                    }
                    val => {
                        result.insert(key, val.to_string());
                    }
                }
            }

            return Ok(result);
        }
        Value::Null => return Ok(Default::default()),
        Value::Bool(_) => "Expected an object for the env constructor parameter, got a boolean.",
        Value::Number(_) => "Expected an object for the env constructor parameter, got a number.",
        Value::String(_) => "Expected an object for the env constructor parameter, got a string.",
        Value::Array(_) => "Expected an object for the env constructor parameter, got an array.",
    };

    Err(ApiError::JsonDecode(msg.to_string()))
}
