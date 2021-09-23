use crate::{error::ApiError, logger::ChannelLogger};
use datamodel::{diagnostics::ValidatedConfiguration, Datamodel};
use napi::threadsafe_function::ThreadsafeFunction;
use opentelemetry::global;
use prisma_models::DatamodelConverter;
use query_core::{executor, schema_builder, BuildMode, QueryExecutor, QuerySchema, QuerySchemaRenderer, TxId};
use request_handlers::{GraphQLSchemaRenderer, GraphQlBody, GraphQlHandler, PrismaResponse, TxInput};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::Level;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// The main engine, that can be cloned between threads when using JavaScript
/// promises.
#[derive(Clone)]
pub struct QueryEngine {
    inner: Arc<RwLock<Inner>>,
}

/// The state of the engine.
pub enum Inner {
    /// Not connected, holding all data to form a connection.
    Builder(EngineBuilder),
    /// A connected engine, holding all data to disconnect and form a new
    /// connection. Allows querying when on this state.
    Connected(ConnectedEngine),
}

/// Holding the information to reconnect the engine if needed.
#[derive(Debug, Clone)]
struct EngineDatamodel {
    datasource_overrides: Vec<(String, String)>,
    ast: Datamodel,
    raw: String,
}

/// Everything needed to connect to the database and have the core running.
pub struct EngineBuilder {
    datamodel: EngineDatamodel,
    config: ValidatedConfiguration,
    logger: ChannelLogger,
    config_dir: PathBuf,
    env: HashMap<String, String>,
}

/// Internal structure for querying and reconnecting with the engine.
pub struct ConnectedEngine {
    datamodel: EngineDatamodel,
    query_schema: Arc<QuerySchema>,
    executor: crate::Executor,
    logger: ChannelLogger,
    config_dir: PathBuf,
    env: HashMap<String, String>,
}

/// Returned from the `serverInfo` method in javascript.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    commit: String,
    version: String,
    primary_connector: Option<String>,
}

impl ConnectedEngine {
    /// The schema AST for Query Engine core.
    pub fn query_schema(&self) -> &Arc<QuerySchema> {
        &self.query_schema
    }

    /// The query executor.
    pub fn executor(&self) -> &(dyn QueryExecutor + Send + Sync) {
        &*self.executor
    }
}

/// Parameters defining the construction of an engine.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructorOptions {
    datamodel: String,
    log_level: String,
    #[serde(default)]
    log_queries: bool,
    #[serde(default)]
    datasource_overrides: BTreeMap<String, String>,
    #[serde(default)]
    env: serde_json::Value,
    #[serde(default)]
    telemetry: TelemetryOptions,
    config_dir: PathBuf,
    #[serde(default)]
    ignore_env_var_errors: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryOptions {
    enabled: bool,
    endpoint: Option<String>,
}

impl QueryEngine {
    /// Parse a validated datamodel and configuration to allow connecting later on.
    pub fn new(opts: ConstructorOptions, log_callback: ThreadsafeFunction<String>) -> crate::Result<Self> {
        set_panic_hook();

        let ConstructorOptions {
            datamodel,
            log_level,
            log_queries,
            datasource_overrides,
            env,
            telemetry,
            config_dir,
            ignore_env_var_errors,
        } = opts;

        let env = stringify_env_values(env)?; // we cannot trust anything JS sends us from process.env
        let overrides: Vec<(_, _)> = datasource_overrides.into_iter().collect();

        let config = if ignore_env_var_errors {
            datamodel::parse_configuration(&datamodel).map_err(|errors| ApiError::conversion(errors, &datamodel))?
        } else {
            datamodel::parse_configuration(&datamodel)
                .and_then(|mut config| {
                    config
                        .subject
                        .resolve_datasource_urls_from_env(&overrides, |key| env.get(key).map(ToString::to_string))?;

                    Ok(config)
                })
                .map_err(|errors| ApiError::conversion(errors, &datamodel))?
        };

        config
            .subject
            .validate_that_one_datasource_is_provided()
            .map_err(|errors| ApiError::conversion(errors, &datamodel))?;

        let ast = datamodel::parse_datamodel(&datamodel)
            .map_err(|errors| ApiError::conversion(errors, &datamodel))?
            .subject;

        let datamodel = EngineDatamodel {
            ast,
            raw: datamodel,
            datasource_overrides: overrides,
        };

        let logger = if telemetry.enabled {
            ChannelLogger::new_with_telemetry(log_callback, telemetry.endpoint)
        } else {
            ChannelLogger::new(&log_level, log_queries, log_callback)
        };

        let builder = EngineBuilder {
            datamodel,
            config,
            logger,
            config_dir,
            env,
        };

        Ok(Self {
            inner: Arc::new(RwLock::new(Inner::Builder(builder))),
        })
    }

    /// Connect to the database, allow queries to be run.
    pub async fn connect(&self) -> crate::Result<()> {
        let mut inner = self.inner.write().await;

        match *inner {
            Inner::Builder(ref builder) => {
                let engine = builder
                    .logger
                    .clone()
                    .with_logging(|| async move {
                        let template = DatamodelConverter::convert(&builder.datamodel.ast);

                        // We only support one data source & generator at the moment, so take the first one (default not exposed yet).
                        let data_source = builder
                            .config
                            .subject
                            .datasources
                            .first()
                            .ok_or_else(|| ApiError::configuration("No valid data source found"))?;

                        let preview_features: Vec<_> = builder.config.subject.preview_features().iter().collect();
                        let url = data_source
                            .load_url_with_config_dir(&builder.config_dir, |key| {
                                builder.env.get(key).map(ToString::to_string)
                            })
                            .map_err(|err| crate::error::ApiError::Conversion(err, builder.datamodel.raw.clone()))?;

                        let (db_name, executor) = executor::load(data_source, &preview_features, &url).await?;
                        let connector = executor.primary_connector();
                        connector.get_connection().await?;

                        // Build internal data model
                        let internal_data_model = template.build(db_name);

                        let query_schema = schema_builder::build(
                            internal_data_model,
                            BuildMode::Modern,
                            true, // enable raw queries
                            data_source.capabilities(),
                            preview_features,
                        );

                        Ok(ConnectedEngine {
                            datamodel: builder.datamodel.clone(),
                            query_schema: Arc::new(query_schema),
                            logger: builder.logger.clone(),
                            executor,
                            config_dir: builder.config_dir.clone(),
                            env: builder.env.clone(),
                        })
                    })
                    .await?;

                *inner = Inner::Connected(engine);

                Ok(())
            }
            Inner::Connected(_) => Err(ApiError::AlreadyConnected),
        }
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    pub async fn disconnect(&self) -> crate::Result<()> {
        let mut inner = self.inner.write().await;

        match *inner {
            Inner::Connected(ref engine) => {
                let config = datamodel::parse_configuration(&engine.datamodel.raw)
                    .map_err(|errors| ApiError::conversion(errors, &engine.datamodel.raw))?;

                let builder = EngineBuilder {
                    datamodel: engine.datamodel.clone(),
                    logger: engine.logger.clone(),
                    config,
                    config_dir: engine.config_dir.clone(),
                    env: engine.env.clone(),
                };

                *inner = Inner::Builder(builder);

                Ok(())
            }
            Inner::Builder(_) => Err(ApiError::NotConnected),
        }
    }

    /// If connected, sends a query to the core and returns the response.
    pub async fn query(
        &self,
        query: GraphQlBody,
        trace: HashMap<String, String>,
        tx_id: Option<String>,
    ) -> crate::Result<PrismaResponse> {
        match *self.inner.read().await {
            Inner::Connected(ref engine) => {
                engine
                    .logger
                    .with_logging(|| async move {
                        let cx = global::get_text_map_propagator(|propagator| propagator.extract(&trace));
                        let span = tracing::span!(Level::TRACE, "query");

                        span.set_parent(cx);

                        let handler = GraphQlHandler::new(engine.executor(), engine.query_schema());
                        Ok(handler.handle(query, tx_id.map(TxId::from)).await)
                    })
                    .await
            }
            Inner::Builder(_) => Err(ApiError::NotConnected),
        }
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    pub async fn start_tx(&self, input: TxInput, trace: HashMap<String, String>) -> crate::Result<String> {
        match *self.inner.read().await {
            Inner::Connected(ref engine) => {
                engine
                    .logger
                    .with_logging(|| async move {
                        let cx = global::get_text_map_propagator(|propagator| propagator.extract(&trace));
                        let span = tracing::span!(Level::TRACE, "query");

                        span.set_parent(cx);

                        match engine.executor().start_tx(input.max_wait, input.timeout).await {
                            Ok(tx_id) => Ok(json!({ "id": tx_id.to_string() }).to_string()),
                            Err(err) => Ok(map_known_error(err)?),
                        }
                    })
                    .await
            }
            Inner::Builder(_) => Err(ApiError::NotConnected),
        }
    }

    /// If connected, attempts to commit a transaction with id `tx_id` in the core.
    pub async fn commit_tx(&self, tx_id: String, trace: HashMap<String, String>) -> crate::Result<String> {
        match *self.inner.read().await {
            Inner::Connected(ref engine) => {
                engine
                    .logger
                    .with_logging(|| async move {
                        let cx = global::get_text_map_propagator(|propagator| propagator.extract(&trace));
                        let span = tracing::span!(Level::TRACE, "query");

                        span.set_parent(cx);

                        match engine.executor().commit_tx(TxId::from(tx_id)).await {
                            Ok(_) => Ok("{}".to_string()),
                            Err(err) => Ok(map_known_error(err)?),
                        }
                    })
                    .await
            }
            Inner::Builder(_) => Err(ApiError::NotConnected),
        }
    }

    /// If connected, attempts to roll back a transaction with id `tx_id` in the core.
    pub async fn rollback_tx(&self, tx_id: String, trace: HashMap<String, String>) -> crate::Result<String> {
        match *self.inner.read().await {
            Inner::Connected(ref engine) => {
                engine
                    .logger
                    .with_logging(|| async move {
                        let cx = global::get_text_map_propagator(|propagator| propagator.extract(&trace));
                        let span = tracing::span!(Level::TRACE, "query");

                        span.set_parent(cx);

                        match engine.executor().rollback_tx(TxId::from(tx_id)).await {
                            Ok(_) => Ok("{}".to_string()),
                            Err(err) => Ok(map_known_error(err)?),
                        }
                    })
                    .await
            }
            Inner::Builder(_) => Err(ApiError::NotConnected),
        }
    }

    /// Loads the query schema. Only available when connected.
    pub async fn sdl_schema(&self) -> crate::Result<String> {
        match *self.inner.read().await {
            Inner::Connected(ref engine) => Ok(GraphQLSchemaRenderer::render(engine.query_schema().clone())),
            Inner::Builder(_) => Err(ApiError::NotConnected),
        }
    }
}

fn map_known_error(err: query_core::CoreError) -> crate::Result<String> {
    let user_error: user_facing_errors::Error = err.into();
    let value = serde_json::to_string(&user_error)?;

    Ok(value)
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

pub fn set_panic_hook() {
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        let payload = info
            .payload()
            .downcast_ref::<String>()
            .map(Clone::clone)
            .unwrap_or_else(|| info.payload().downcast_ref::<&str>().unwrap().to_string());

        match info.location() {
            Some(location) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    message = "PANIC",
                    reason = payload.as_str(),
                    file = location.file(),
                    line = location.line(),
                    column = location.column(),
                );
            }
            None => {
                tracing::event!(tracing::Level::ERROR, message = "PANIC", reason = payload.as_str());
            }
        }

        original_hook(info)
    }));
}
