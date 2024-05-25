// `clippy::empty_docs` is required because of the `tsify` crate.
#![allow(unused_imports, clippy::empty_docs)]

use crate::error::ApiError;
use query_core::{protocol::EngineProtocol, schema::QuerySchema, QueryExecutor};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};

#[cfg(target_arch = "wasm32")]
use tsify::Tsify;

/// The state of the engine.
pub enum Inner {
    /// Not connected, holding all data to form a connection.
    Builder(EngineBuilder),
    /// A connected engine, holding all data to disconnect and form a new
    /// connection. Allows querying when on this state.
    Connected(ConnectedEngine),
}

impl Inner {
    /// Returns a builder if the engine is not connected
    pub fn as_builder(&self) -> crate::Result<&EngineBuilder> {
        match self {
            Inner::Builder(ref builder) => Ok(builder),
            Inner::Connected(_) => Err(ApiError::AlreadyConnected),
        }
    }

    /// Returns the engine if connected
    pub fn as_engine(&self) -> crate::Result<&ConnectedEngine> {
        match self {
            Inner::Builder(_) => Err(ApiError::NotConnected),
            Inner::Connected(ref engine) => Ok(engine),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct EngineBuilderNative {
    pub config_dir: PathBuf,
    pub env: HashMap<String, String>,
}

/// Everything needed to connect to the database and have the core running.
pub struct EngineBuilder {
    pub schema: Arc<psl::ValidatedSchema>,
    pub engine_protocol: EngineProtocol,

    #[cfg(not(target_arch = "wasm32"))]
    pub native: EngineBuilderNative,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct ConnectedEngineNative {
    pub config_dir: PathBuf,
    pub env: HashMap<String, String>,
    pub metrics: Option<query_engine_metrics::MetricRegistry>,
}

/// Internal structure for querying and reconnecting with the engine.
pub struct ConnectedEngine {
    pub schema: Arc<psl::ValidatedSchema>,
    pub query_schema: Arc<QuerySchema>,
    pub executor: crate::Executor,
    pub engine_protocol: EngineProtocol,

    #[cfg(not(target_arch = "wasm32"))]
    pub native: ConnectedEngineNative,
}

impl ConnectedEngine {
    /// The schema AST for Query Engine core.
    pub fn query_schema(&self) -> &Arc<QuerySchema> {
        &self.query_schema
    }

    /// The query executor.
    pub fn executor(&self) -> &(dyn QueryExecutor + Send + Sync) {
        self.executor.as_ref()
    }

    pub fn engine_protocol(&self) -> EngineProtocol {
        self.engine_protocol
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructorOptionsNative {
    #[serde(default)]
    pub datasource_overrides: BTreeMap<String, String>,
    pub config_dir: PathBuf,
    #[serde(default)]
    pub env: serde_json::Value,
    #[serde(default)]
    pub ignore_env_var_errors: bool,
    #[serde(default)]
    pub engine_protocol: Option<EngineProtocol>,
}

/// Parameters defining the construction of an engine.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct ConstructorOptions {
    pub datamodel: String,
    pub log_level: String,
    #[serde(default)]
    pub log_queries: bool,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(flatten)]
    pub native: ConstructorOptionsNative,
}

pub fn map_known_error(err: query_core::CoreError) -> crate::Result<String> {
    let user_error: user_facing_errors::Error = err.into();
    let value = serde_json::to_string(&user_error)?;

    Ok(value)
}

pub fn stringify_env_values(origin: serde_json::Value) -> crate::Result<HashMap<String, String>> {
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
