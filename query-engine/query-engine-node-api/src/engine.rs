use crate::{engine::executor::TransactionOptions, error::ApiError, log_callback::LogCallback, logger::Logger};
use futures::FutureExt;
use psl::PreviewFeature;
use query_core::{
    executor,
    protocol::EngineProtocol,
    schema::{QuerySchema, QuerySchemaRenderer},
    schema_builder, telemetry, QueryExecutor, TxId,
};
use query_engine_metrics::{MetricFormat, MetricRegistry};
use request_handlers::{dmmf, GraphQLSchemaRenderer, RequestBody, RequestHandler};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    panic::AssertUnwindSafe,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{field, instrument::WithSubscriber, Instrument, Span};
use tracing_subscriber::filter::LevelFilter;
use user_facing_errors::Error;

use napi::{Env, JsFunction, JsUnknown};
use napi_derive::napi;

/// The main query engine used by JS
#[napi]
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
    engine_protocol: EngineProtocol,
}

/// Internal structure for querying and reconnecting with the engine.
struct ConnectedEngine {
    schema: Arc<psl::ValidatedSchema>,
    query_schema: Arc<QuerySchema>,
    executor: crate::Executor,
    config_dir: PathBuf,
    env: HashMap<String, String>,
    metrics: Option<MetricRegistry>,
    engine_protocol: EngineProtocol,
}

/// Returned from the `serverInfo` method in javascript.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    commit: String,
    version: String,
    primary_connector: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetricOptions {
    format: MetricFormat,
    #[serde(default)]
    global_labels: HashMap<String, String>,
}

impl MetricOptions {
    fn is_json_format(&self) -> bool {
        self.format == MetricFormat::Json
    }
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

    pub fn engine_protocol(&self) -> EngineProtocol {
        self.engine_protocol
    }
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
    #[serde(default)]
    engine_protocol: Option<EngineProtocol>,
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

#[napi]
impl QueryEngine {
    /// Parse a validated datamodel and configuration to allow connecting later on.
    #[napi(constructor)]
    pub fn new(napi_env: Env, options: JsUnknown, callback: JsFunction) -> napi::Result<Self> {
        let log_callback = LogCallback::new(napi_env, callback)?;
        log_callback.unref(&napi_env)?;

        let ConstructorOptions {
            datamodel,
            log_level,
            log_queries,
            datasource_overrides,
            env,
            config_dir,
            ignore_env_var_errors,
            engine_protocol,
        } = napi_env.from_js_value(options)?;

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

        let enable_metrics = config.preview_features().contains(PreviewFeature::Metrics);
        let enable_tracing = config.preview_features().contains(PreviewFeature::Tracing);
        let engine_protocol =
            engine_protocol.unwrap_or_else(
                || match config.preview_features().contains(PreviewFeature::JsonProtocol) {
                    true => EngineProtocol::Json,
                    false => EngineProtocol::Graphql,
                },
            );

        let builder = EngineBuilder {
            schema: Arc::new(schema),
            config_dir,
            engine_protocol,
            env,
        };

        let log_level = log_level.parse::<LevelFilter>().unwrap();
        let logger = Logger::new(log_queries, log_level, log_callback, enable_metrics, enable_tracing);

        // Describe metrics adds all the descriptions and default values for our metrics
        // this needs to run once our metrics pipeline has been configured and it needs to
        // use the correct logging subscriber(our dispatch) so that the metrics recorder recieves
        // it
        if enable_metrics {
            napi_env.execute_tokio_future(
                async {
                    query_engine_metrics::describe_metrics();
                    Ok(())
                }
                .with_subscriber(logger.dispatcher()),
                |&mut _env, _data| Ok(()),
            )?;
        }

        Ok(Self {
            inner: RwLock::new(Inner::Builder(builder)),
            logger,
        })
    }

    /// Connect to the database, allow queries to be run.
    #[napi]
    pub async fn connect(&self, trace: String) -> napi::Result<()> {
        let dispatcher = self.logger.dispatcher();

        async_panic_to_js_error(async {
            let span = tracing::info_span!("prisma:engine:connect");
            let _ = telemetry::helpers::set_parent_context_from_json_str(&span, &trace);

            let mut inner = self.inner.write().await;
            let builder = inner.as_builder()?;

            let engine = async move {
                // We only support one data source & generator at the moment, so take the first one (default not exposed yet).
                let data_source = builder
                    .schema
                    .configuration
                    .datasources
                    .first()
                    .ok_or_else(|| ApiError::configuration("No valid data source found"))?;

                let preview_features = builder.schema.configuration.preview_features();
                let url = data_source
                    .load_url_with_config_dir(&builder.config_dir, |key| builder.env.get(key).map(ToString::to_string))
                    .map_err(|err| crate::error::ApiError::Conversion(err, builder.schema.db.source().to_owned()))?;

                let executor = executor::load(data_source, preview_features, &url).await?;
                let connector = executor.primary_connector();
                connector.get_connection().await?;

                // Build internal data model
                let internal_data_model = prisma_models::convert(Arc::clone(&builder.schema));

                let enable_raw_queries = true;
                let query_schema = schema_builder::build(internal_data_model, enable_raw_queries);

                Ok(ConnectedEngine {
                    schema: builder.schema.clone(),
                    query_schema: Arc::new(query_schema),
                    executor,
                    config_dir: builder.config_dir.clone(),
                    env: builder.env.clone(),
                    metrics: self.logger.metrics(),
                    engine_protocol: builder.engine_protocol,
                }) as crate::Result<ConnectedEngine>
            }
            .instrument(span)
            .await?;

            *inner = Inner::Connected(engine);

            Ok(())
        })
        .with_subscriber(dispatcher)
        .await?;

        Ok(())
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    #[napi]
    pub async fn disconnect(&self, trace: String) -> napi::Result<()> {
        let dispatcher = self.logger.dispatcher();

        async_panic_to_js_error(async {
            let span = tracing::info_span!("prisma:engine:disconnect");
            let _ = telemetry::helpers::set_parent_context_from_json_str(&span, &trace);

            async {
                let mut inner = self.inner.write().await;
                let engine = inner.as_engine()?;

                let builder = EngineBuilder {
                    schema: engine.schema.clone(),
                    config_dir: engine.config_dir.clone(),
                    env: engine.env.clone(),
                    engine_protocol: engine.engine_protocol(),
                };

                *inner = Inner::Builder(builder);

                Ok(())
            }
            .instrument(span)
            .await
        })
        .with_subscriber(dispatcher)
        .await
    }

    /// If connected, sends a query to the core and returns the response.
    #[napi]
    pub async fn query(&self, body: String, trace: String, tx_id: Option<String>) -> napi::Result<String> {
        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let query = RequestBody::try_from_str(&body, engine.engine_protocol())?;

            let dispatcher = self.logger.dispatcher();

            async move {
                let span = if tx_id.is_none() {
                    tracing::info_span!("prisma:engine", user_facing = true)
                } else {
                    Span::none()
                };

                let trace_id = telemetry::helpers::set_parent_context_from_json_str(&span, &trace);

                let handler = RequestHandler::new(engine.executor(), engine.query_schema(), engine.engine_protocol());
                let response = handler
                    .handle(query, tx_id.map(TxId::from), trace_id)
                    .instrument(span)
                    .await;

                Ok(serde_json::to_string(&response)?)
            }
            .with_subscriber(dispatcher)
            .await
        })
        .await
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    #[napi]
    pub async fn start_transaction(&self, input: String, trace: String) -> napi::Result<String> {
        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let dispatcher = self.logger.dispatcher();

            async move {
                let span = tracing::info_span!("prisma:engine:itx_runner", user_facing = true, itx_id = field::Empty);
                telemetry::helpers::set_parent_context_from_json_str(&span, &trace);

                let tx_opts: TransactionOptions = serde_json::from_str(&input)?;
                match engine
                    .executor()
                    .start_tx(engine.query_schema().clone(), engine.engine_protocol(), tx_opts)
                    .instrument(span)
                    .await
                {
                    Ok(tx_id) => Ok(json!({ "id": tx_id.to_string() }).to_string()),
                    Err(err) => Ok(map_known_error(err)?),
                }
            }
            .with_subscriber(dispatcher)
            .await
        })
        .await
    }

    /// If connected, attempts to commit a transaction with id `tx_id` in the core.
    #[napi]
    pub async fn commit_transaction(&self, tx_id: String, _trace: String) -> napi::Result<String> {
        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let dispatcher = self.logger.dispatcher();

            async move {
                match engine.executor().commit_tx(TxId::from(tx_id)).await {
                    Ok(_) => Ok("{}".to_string()),
                    Err(err) => Ok(map_known_error(err)?),
                }
            }
            .with_subscriber(dispatcher)
            .await
        })
        .await
    }

    #[napi]
    pub async fn dmmf(&self) -> napi::Result<String> {
        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;
            let dmmf = dmmf::render_dmmf(engine.query_schema.clone());

            Ok(serde_json::to_string(&dmmf)?)
        })
        .await
    }

    /// If connected, attempts to roll back a transaction with id `tx_id` in the core.
    #[napi]
    pub async fn rollback_transaction(&self, tx_id: String, _trace: String) -> napi::Result<String> {
        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let dispatcher = self.logger.dispatcher();

            async move {
                match engine.executor().rollback_tx(TxId::from(tx_id)).await {
                    Ok(_) => Ok("{}".to_string()),
                    Err(err) => Ok(map_known_error(err)?),
                }
            }
            .with_subscriber(dispatcher)
            .await
        })
        .await
    }

    /// Loads the query schema. Only available when connected.
    #[napi]
    pub async fn sdl_schema(&self) -> napi::Result<String> {
        async_panic_to_js_error(async move {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            Ok(GraphQLSchemaRenderer::render(engine.query_schema().clone()))
        })
        .await
    }

    #[napi]
    pub async fn metrics(&self, json_options: String) -> napi::Result<String> {
        async_panic_to_js_error(async move {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;
            let options: MetricOptions = serde_json::from_str(&json_options)?;

            if let Some(metrics) = &engine.metrics {
                if options.is_json_format() {
                    let engine_metrics = metrics.to_json(options.global_labels);
                    let res = serde_json::to_string(&engine_metrics)?;
                    Ok(res)
                } else {
                    Ok(metrics.to_prometheus(options.global_labels))
                }
            } else {
                Err(ApiError::Configuration(
                    "Metrics is not enabled. First set it in the preview features.".to_string(),
                )
                .into())
            }
        })
        .await
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

async fn async_panic_to_js_error<F, R>(fut: F) -> napi::Result<R>
where
    F: Future<Output = napi::Result<R>>,
{
    match AssertUnwindSafe(fut).catch_unwind().await {
        Ok(result) => result,
        Err(err) => match Error::extract_panic_message(err) {
            Some(message) => Err(napi::Error::from_reason(format!("PANIC: {message}"))),
            None => Err(napi::Error::from_reason("PANIC: unknown panic".to_string())),
        },
    }
}
