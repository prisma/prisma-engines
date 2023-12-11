#![allow(dead_code)]
#![allow(unused_variables)]

use crate::{
    error::ApiError,
    logger::{LogCallback, Logger},
};
use driver_adapters::JsObject;
use js_sys::Function as JsFunction;
use query_core::{
    protocol::EngineProtocol,
    schema::{self},
    telemetry, TransactionOptions, TxId,
};
use query_engine_common::engine::{
    map_known_error, stringify_env_values, ConnectedEngine, ConstructorOptions, EngineBuilder, Inner,
};
use request_handlers::ConnectorMode;
use request_handlers::{load_executor, RequestBody, RequestHandler};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{field, instrument::WithSubscriber, Instrument, Span};
use tracing_subscriber::filter::LevelFilter;
use wasm_bindgen::prelude::wasm_bindgen;

/// Returned from the `serverInfo` method in javascript.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    commit: String,
    version: String,
    primary_connector: Option<String>,
}

#[wasm_bindgen]
pub struct QueryEngine {
    inner: RwLock<Inner>,
    logger: Logger,
    connector_mode: ConnectorMode,
}

#[wasm_bindgen]
impl QueryEngine {
    /// Parse a validated datamodel and configuration to allow connecting later on.
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: ConstructorOptions,
        callback: JsFunction,
        maybe_adapter: Option<JsObject>,
    ) -> Result<QueryEngine, wasm_bindgen::JsError> {
        let log_callback = LogCallback(callback);

        let ConstructorOptions {
            datamodel,
            log_level,
            log_queries,
            datasource_overrides,
            env,
            config_dir,
            ignore_env_var_errors,
            engine_protocol,
        } = options;

        let env = stringify_env_values(env)?; // we cannot trust anything JS sends us from process.env
        let overrides: Vec<(_, _)> = datasource_overrides.into_iter().collect();

        let mut schema = psl::validate(datamodel.into());
        let config = &mut schema.configuration;
        let preview_features = config.preview_features();

        if let Some(adapter) = maybe_adapter {
            let js_queryable = driver_adapters::from_js(adapter);

            sql_connector::activate_driver_adapter(Arc::new(js_queryable));

            let provider_name = schema.connector.provider_name();
            tracing::info!("Registered driver adapter for {provider_name}.");
        }

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

        // Telemetry panics on timings if preview feature is enabled
        let enable_tracing = false; // config.preview_features().contains(PreviewFeature::Tracing);
        let engine_protocol = engine_protocol.unwrap_or(EngineProtocol::Json);

        let builder = EngineBuilder {
            schema: Arc::new(schema),
            config_dir,
            engine_protocol,
            env,
        };

        let log_level = log_level.parse::<LevelFilter>().unwrap();
        let logger = Logger::new(log_queries, log_level, log_callback, enable_tracing);

        let connector_mode = ConnectorMode::Js;

        Ok(Self {
            inner: RwLock::new(Inner::Builder(builder)),
            logger,
            connector_mode,
        })
    }

    /// Connect to the database, allow queries to be run.
    #[wasm_bindgen]
    pub async fn connect(&self, trace: String) -> Result<(), wasm_bindgen::JsError> {
        let dispatcher = self.logger.dispatcher();

        async {
            let span = tracing::info_span!("prisma:engine:connect");
            let _ = telemetry::helpers::set_parent_context_from_json_str(&span, &trace);

            let mut inner = self.inner.write().await;
            let builder = inner.as_builder()?;
            let arced_schema = Arc::clone(&builder.schema);
            let arced_schema_2 = Arc::clone(&builder.schema);

            let url = {
                let data_source = builder
                    .schema
                    .configuration
                    .datasources
                    .first()
                    .ok_or_else(|| ApiError::configuration("No valid data source found"))?;
                data_source
                    .load_url_with_config_dir(&builder.config_dir, |key| builder.env.get(key).map(ToString::to_string))
                    .map_err(|err| crate::error::ApiError::Conversion(err, builder.schema.db.source().to_owned()))?
            };

            let engine = async move {
                // We only support one data source & generator at the moment, so take the first one (default not exposed yet).
                let data_source = arced_schema
                    .configuration
                    .datasources
                    .first()
                    .ok_or_else(|| ApiError::configuration("No valid data source found"))?;

                let preview_features = arced_schema.configuration.preview_features();

                let executor = load_executor(self.connector_mode, data_source, preview_features, &url).await?;
                let connector = executor.primary_connector();

                let conn_span = tracing::info_span!(
                    "prisma:engine:connection",
                    user_facing = true,
                    "db.type" = connector.name(),
                );

                connector.get_connection().instrument(conn_span).await?;

                let query_schema_span = tracing::info_span!("prisma:engine:schema");
                let query_schema = query_schema_span.in_scope(|| schema::build(arced_schema_2, true));

                Ok(ConnectedEngine {
                    schema: builder.schema.clone(),
                    query_schema: Arc::new(query_schema),
                    executor,
                    config_dir: builder.config_dir.clone(),
                    env: builder.env.clone(),
                    engine_protocol: builder.engine_protocol,
                }) as crate::Result<ConnectedEngine>
            }
            .instrument(span)
            .await?;

            *inner = Inner::Connected(engine);

            Ok(())
        }
        .with_subscriber(dispatcher)
        .await
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    #[wasm_bindgen]
    pub async fn disconnect(&self, trace: String) -> Result<(), wasm_bindgen::JsError> {
        let dispatcher = self.logger.dispatcher();

        async {
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
        }
        .with_subscriber(dispatcher)
        .await
    }

    /// If connected, sends a query to the core and returns the response.
    #[wasm_bindgen]
    pub async fn query(
        &self,
        body: String,
        trace: String,
        tx_id: Option<String>,
    ) -> Result<String, wasm_bindgen::JsError> {
        let dispatcher = self.logger.dispatcher();

        async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let query = RequestBody::try_from_str(&body, engine.engine_protocol())?;

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
            .await
        }
        .with_subscriber(dispatcher)
        .await
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    #[wasm_bindgen(js_name = startTransaction)]
    pub async fn start_transaction(&self, input: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
        let inner = self.inner.read().await;
        let engine = inner.as_engine()?;
        let dispatcher = self.logger.dispatcher();

        async move {
            let span = tracing::info_span!("prisma:engine:itx_runner", user_facing = true, itx_id = field::Empty);

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
    }

    /// If connected, attempts to commit a transaction with id `tx_id` in the core.
    #[wasm_bindgen(js_name = commitTransaction)]
    pub async fn commit_transaction(&self, tx_id: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
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
    }

    /// If connected, attempts to roll back a transaction with id `tx_id` in the core.
    #[wasm_bindgen(js_name = rollbackTransaction)]
    pub async fn rollback_transaction(&self, tx_id: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
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
    }

    #[wasm_bindgen]
    pub async fn metrics(&self, json_options: String) -> Result<(), wasm_bindgen::JsError> {
        Err(ApiError::configuration("Metrics is not enabled in Wasm.").into())
    }
}
