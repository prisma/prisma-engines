use crate::{error::ApiError, logger::Logger};
use futures::FutureExt;
use napi::{threadsafe_function::ThreadSafeCallContext, Env, JsFunction, JsObject, JsUnknown};
use napi_derive::napi;
use prisma_metrics::{MetricFormat, WithMetricsInstrumentation};
use psl::PreviewFeature;
use quaint::connector::ExternalConnector;
use query_core::{protocol::EngineProtocol, relation_load_strategy, schema, TransactionOptions, TxId};
use query_engine_common::{
    engine::{
        map_known_error, stringify_env_values, ConnectedEngine, ConnectedEngineNative, ConstructorOptions,
        ConstructorOptionsNative, EngineBuilder, EngineBuilderNative, Inner,
    },
    tracer::start_trace,
};
use request_handlers::{load_executor, render_graphql_schema, ConnectorKind, RequestBody, RequestHandler};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, future::Future, marker::PhantomData, panic::AssertUnwindSafe, sync::Arc};
use tokio::sync::RwLock;
use tracing_futures::{Instrument, WithSubscriber};
use tracing_subscriber::filter::LevelFilter;
use user_facing_errors::Error;

enum ConnectorMode {
    Rust,
    Js { adapter: Arc<dyn ExternalConnector> },
}

/// The main query engine used by JS
#[napi]
pub struct QueryEngine {
    connector_mode: ConnectorMode,
    inner: RwLock<Inner>,
    logger: Logger,
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

#[napi]
impl QueryEngine {
    /// Parse a validated datamodel and configuration to allow connecting later on.
    /// Note: any new method added to this struct should be added to
    /// `query_engine_node_api::node_drivers::engine::QueryEngineNodeDrivers` as well.
    /// Unfortunately the `#[napi]` macro does not support deriving traits.
    #[napi(constructor)]
    pub fn new(
        napi_env: Env,
        options: JsUnknown,
        callback: JsFunction,
        maybe_adapter: Option<JsObject>,
    ) -> napi::Result<Self> {
        let mut log_callback = callback.create_threadsafe_function(0usize, |ctx: ThreadSafeCallContext<String>| {
            Ok(vec![ctx.env.create_string(&ctx.value)?])
        })?;
        log_callback.unref(&napi_env)?;

        let ConstructorOptions {
            datamodel,
            log_level,
            log_queries,
            native,
        } = napi_env.from_js_value(options).expect(
            r###"
            Failed to deserialize constructor options.

            This usually happens when the javascript object passed to the constructor is missing
            properties for the ConstructorOptions fields that must have some value.

            If you set some of these in javascript through environment variables, make sure there are
            values for data_model, log_level, and any field that is not Option<T>
            "###,
        );

        let ConstructorOptionsNative {
            datasource_overrides,
            config_dir,
            env,
            ignore_env_var_errors,
            engine_protocol,
        } = native;

        let env = stringify_env_values(env)?; // we cannot trust anything JS sends us from process.env
        let overrides: Vec<(_, _)> = datasource_overrides.into_iter().collect();

        let mut schema = psl::validate(datamodel.into());
        let config = &mut schema.configuration;
        let preview_features = config.preview_features();

        let mut connector_mode = ConnectorMode::Rust;

        if !preview_features.contains(PreviewFeature::DriverAdapters) {
            tracing::info!(
                "Please enable the {} preview feature to use driver adapters.",
                PreviewFeature::DriverAdapters
            );
        } else {
            #[cfg(feature = "driver-adapters")]
            if let Some(adapter) = maybe_adapter {
                let js_queryable = driver_adapters::from_js(adapter);

                connector_mode = ConnectorMode::Js {
                    adapter: Arc::new(js_queryable),
                };

                let provider_name = schema.connector.provider_name();
                tracing::info!("Registered driver adapter for {provider_name}.");
            }
        }

        let connector_mode = connector_mode;

        schema
            .diagnostics
            .to_result()
            .map_err(|err| ApiError::conversion(err, schema.db.source_assert_single()))?;

        config
            .resolve_datasource_urls_query_engine(
                &overrides,
                |key| env.get(key).map(ToString::to_string),
                ignore_env_var_errors,
            )
            .map_err(|err| ApiError::conversion(err, schema.db.source_assert_single()))?;

        config
            .validate_that_one_datasource_is_provided()
            .map_err(|errors| ApiError::conversion(errors, schema.db.source_assert_single()))?;

        let enable_metrics = config.preview_features().contains(PreviewFeature::Metrics);
        let enable_tracing = config.preview_features().contains(PreviewFeature::Tracing);
        let engine_protocol = engine_protocol.unwrap_or(EngineProtocol::Json);

        let builder = EngineBuilder {
            schema: Arc::new(schema),
            engine_protocol,
            native: EngineBuilderNative { config_dir, env },
        };

        let log_level = log_level.parse::<LevelFilter>().unwrap();
        let logger = Logger::new(log_queries, log_level, log_callback, enable_metrics, enable_tracing);

        Ok(Self {
            connector_mode,
            inner: RwLock::new(Inner::Builder(builder)),
            logger,
        })
    }

    /// Connect to the database, allow queries to be run.
    #[napi]
    pub async fn connect(&self, trace: String, request_id: String) -> napi::Result<()> {
        let dispatcher = self.logger.dispatcher();
        let recorder = self.logger.recorder();
        let exporter = self.logger.exporter();

        async_panic_to_js_error(async {
            let span = tracing::info_span!(
                "prisma:engine:connect",
                user_facing = true,
                request_id = tracing::field::Empty,
            );
            start_trace(&request_id, &trace, &span, &exporter).await?;

            let mut inner = self.inner.write().await;
            let builder = inner.as_builder()?;
            let arced_schema = Arc::clone(&builder.schema);
            let arced_schema_2 = Arc::clone(&builder.schema);

            let engine = async move {
                // We only support one data source & generator at the moment, so take the first one (default not exposed yet).
                let data_source = arced_schema
                    .configuration
                    .datasources
                    .first()
                    .ok_or_else(|| ApiError::configuration("No valid data source found"))?;

                let preview_features = arced_schema.configuration.preview_features();

                let executor_fut = async {
                    let connector_kind = match self.connector_mode {
                        ConnectorMode::Rust => {
                            let url = data_source
                                .load_url_with_config_dir(&builder.native.config_dir, |key| {
                                    builder.native.env.get(key).map(ToString::to_string)
                                })
                                .map_err(|err| {
                                    crate::error::ApiError::Conversion(
                                        err,
                                        builder.schema.db.source_assert_single().to_owned(),
                                    )
                                })?;
                            ConnectorKind::Rust {
                                url,
                                datasource: data_source,
                            }
                        }
                        ConnectorMode::Js { ref adapter } => ConnectorKind::Js {
                            adapter: Arc::clone(adapter),
                            _phantom: PhantomData,
                        },
                    };
                    let executor = load_executor(connector_kind, preview_features).await?;
                    let connector = executor.primary_connector();

                    let conn_span = tracing::info_span!(
                        "prisma:engine:connection",
                        user_facing = true,
                        "db.system" = connector.name(),
                    );

                    let conn = connector.get_connection().instrument(conn_span).await?;
                    let database_version = conn.version().await;

                    crate::Result::<_>::Ok((executor, database_version))
                };

                let query_schema_span = tracing::info_span!("prisma:engine:schema");
                let query_schema_fut = tokio::runtime::Handle::current()
                    .spawn_blocking(move || {
                        let enable_raw_queries = true;
                        schema::build(arced_schema_2, enable_raw_queries)
                    })
                    .instrument(query_schema_span);

                let (query_schema, executor_with_db_version) = tokio::join!(query_schema_fut, executor_fut);
                let (executor, db_version) = executor_with_db_version?;

                let query_schema = query_schema.unwrap().with_db_version_supports_join_strategy(
                    relation_load_strategy::db_version_supports_joins_strategy(db_version)?,
                );

                Ok(ConnectedEngine {
                    schema: builder.schema.clone(),
                    query_schema: Arc::new(query_schema),
                    executor,
                    engine_protocol: builder.engine_protocol,
                    native: ConnectedEngineNative {
                        config_dir: builder.native.config_dir.clone(),
                        env: builder.native.env.clone(),
                        metrics: self.logger.metrics(),
                    },
                }) as crate::Result<ConnectedEngine>
            }
            .instrument(span)
            .await?;

            *inner = Inner::Connected(engine);

            Ok(())
        })
        .with_subscriber(dispatcher)
        .with_optional_recorder(recorder)
        .await?;

        Ok(())
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    #[napi]
    pub async fn disconnect(&self, trace: String, request_id: String) -> napi::Result<()> {
        let dispatcher = self.logger.dispatcher();
        let recorder = self.logger.recorder();
        let exporter = self.logger.exporter();

        async_panic_to_js_error(async {
            let span = tracing::info_span!(
                "prisma:engine:disconnect",
                user_facing = true,
                request_id = tracing::field::Empty,
            );
            start_trace(&request_id, &trace, &span, &exporter).await?;

            // TODO: when using Node Drivers, we need to call Driver::close() here.

            async {
                let mut inner = self.inner.write().await;
                let engine = inner.as_engine()?;

                let builder = EngineBuilder {
                    schema: engine.schema.clone(),
                    engine_protocol: engine.engine_protocol(),
                    native: EngineBuilderNative {
                        config_dir: engine.native.config_dir.clone(),
                        env: engine.native.env.clone(),
                    },
                };

                *inner = Inner::Builder(builder);

                Ok(())
            }
            .instrument(span)
            .await
        })
        .with_subscriber(dispatcher)
        .with_optional_recorder(recorder)
        .await
    }

    /// If connected, sends a query to the core and returns the response.
    #[napi]
    pub async fn query(
        &self,
        body: String,
        trace: String,
        tx_id: Option<String>,
        request_id: String,
    ) -> napi::Result<String> {
        let dispatcher = self.logger.dispatcher();
        let recorder = self.logger.recorder();
        let exporter = self.logger.exporter();

        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let query = RequestBody::try_from_str(&body, engine.engine_protocol())?;

            let span = tracing::info_span!(
                "prisma:engine:query",
                user_facing = true,
                request_id = tracing::field::Empty,
            );
            let trace_parent = start_trace(&request_id, &trace, &span, &exporter).await?;

            async move {
                let handler = RequestHandler::new(engine.executor(), engine.query_schema(), engine.engine_protocol());
                let mut response = handler.handle(query, tx_id.map(TxId::from), trace_parent).await;

                response.set_extension("request_id".to_owned(), json!(request_id));

                let serde_span = tracing::info_span!("prisma:engine:response_json_serialization", user_facing = true);
                Ok(serde_span.in_scope(|| serde_json::to_string(&response))?)
            }
            .instrument(span)
            .await
        })
        .with_subscriber(dispatcher)
        .with_optional_recorder(recorder)
        .await
    }

    /// Fetch the spans associated with a [`RequestId`]
    #[napi]
    pub async fn trace(&self, request_id: String) -> napi::Result<Option<String>> {
        async_panic_to_js_error(async {
            let request_id = request_id
                .parse()
                .map_err(|_| ApiError::Decode("invalid request id".into()))?;

            let exporter = self.logger.exporter();

            Ok(exporter
                .stop_capturing(request_id)
                .await
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?)
        })
        .with_subscriber(self.logger.dispatcher())
        .with_optional_recorder(self.logger.recorder())
        .await
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    #[napi]
    pub async fn start_transaction(&self, input: String, trace: String, request_id: String) -> napi::Result<String> {
        let dispatcher = self.logger.dispatcher();
        let recorder = self.logger.recorder();
        let exporter = self.logger.exporter();

        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;
            let tx_opts: TransactionOptions = serde_json::from_str(&input)?;

            let span = tracing::info_span!(
                "prisma:engine:start_transaction",
                user_facing = true,
                request_id = tracing::field::Empty,
            );
            start_trace(&request_id, &trace, &span, &exporter).await?;

            async move {
                match engine
                    .executor()
                    .start_tx(engine.query_schema().clone(), engine.engine_protocol(), tx_opts)
                    .await
                {
                    Ok(tx_id) => Ok(json!({ "id": tx_id.to_string() }).to_string()),
                    Err(err) => Ok(map_known_error(err)?),
                }
            }
            .instrument(span)
            .await
        })
        .with_subscriber(dispatcher)
        .with_optional_recorder(recorder)
        .await
    }

    /// If connected, attempts to commit a transaction with id `tx_id` in the core.
    #[napi]
    pub async fn commit_transaction(&self, tx_id: String, trace: String, request_id: String) -> napi::Result<String> {
        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let dispatcher = self.logger.dispatcher();
            let recorder = self.logger.recorder();
            let exporter = self.logger.exporter();

            async move {
                let span = tracing::info_span!(
                    "prisma:engine:commit_transaction",
                    user_facing = true,
                    request_id = tracing::field::Empty,
                );
                start_trace(&request_id, &trace, &span, &exporter).await?;

                match engine.executor().commit_tx(TxId::from(tx_id)).instrument(span).await {
                    Ok(_) => Ok("{}".to_string()),
                    Err(err) => Ok(map_known_error(err)?),
                }
            }
            .with_subscriber(dispatcher)
            .with_optional_recorder(recorder)
            .await
        })
        .await
    }

    /// If connected, attempts to roll back a transaction with id `tx_id` in the core.
    #[napi]
    pub async fn rollback_transaction(&self, tx_id: String, trace: String, request_id: String) -> napi::Result<String> {
        async_panic_to_js_error(async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let dispatcher = self.logger.dispatcher();
            let recorder = self.logger.recorder();
            let exporter = self.logger.exporter();

            async move {
                let span = tracing::info_span!(
                    "prisma:engine:rollback_transaction",
                    user_facing = true,
                    request_id = tracing::field::Empty,
                );
                start_trace(&request_id, &trace, &span, &exporter).await?;

                match engine.executor().rollback_tx(TxId::from(tx_id)).instrument(span).await {
                    Ok(_) => Ok("{}".to_string()),
                    Err(err) => Ok(map_known_error(err)?),
                }
            }
            .with_subscriber(dispatcher)
            .with_optional_recorder(recorder)
            .await
        })
        .await
    }

    /// Loads the query schema. Only available when connected.
    #[napi]
    pub async fn sdl_schema(&self) -> napi::Result<String> {
        let dispatcher = self.logger.dispatcher();
        let recorder = self.logger.recorder();

        async_panic_to_js_error(async move {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            Ok(render_graphql_schema(engine.query_schema()))
        })
        .with_subscriber(dispatcher)
        .with_optional_recorder(recorder)
        .await
    }

    #[napi]
    pub async fn metrics(&self, json_options: String) -> napi::Result<String> {
        let dispatcher = self.logger.dispatcher();
        let recorder = self.logger.recorder();

        async_panic_to_js_error(async move {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;
            let options: MetricOptions = serde_json::from_str(&json_options)?;

            if let Some(metrics) = &engine.native.metrics {
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
        .with_subscriber(dispatcher)
        .with_optional_recorder(recorder)
        .await
    }
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
