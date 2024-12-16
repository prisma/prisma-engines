#![allow(dead_code)]
#![allow(unused_variables)]

use crate::{
    error::ApiError,
    logger::{LogCallback, Logger},
};
use driver_adapters::JsObject;
use js_sys::Function as JsFunction;
use psl::{ConnectorRegistry, PreviewFeature};
use quaint::connector::ExternalConnector;
use query_core::{
    protocol::EngineProtocol,
    relation_load_strategy,
    schema::{self},
    TransactionOptions, TxId,
};
use query_engine_common::{
    engine::{map_known_error, ConnectedEngine, ConstructorOptions, EngineBuilder, Inner},
    tracer::start_trace,
};
use request_handlers::ConnectorKind;
use request_handlers::{load_executor, RequestBody, RequestHandler};
use serde_json::json;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::RwLock;
use tracing::{instrument::WithSubscriber, Instrument, Level};
use tracing_subscriber::filter::LevelFilter;
use wasm_bindgen::prelude::wasm_bindgen;

const CONNECTOR_REGISTRY: ConnectorRegistry<'_> = &[
    #[cfg(feature = "postgresql")]
    psl::builtin_connectors::POSTGRES,
    #[cfg(feature = "mysql")]
    psl::builtin_connectors::MYSQL,
    #[cfg(feature = "sqlite")]
    psl::builtin_connectors::SQLITE,
];

/// The main query engine used by JS
#[wasm_bindgen]
pub struct QueryEngine {
    inner: RwLock<Inner>,
    adapter: Arc<dyn ExternalConnector>,
    logger: Logger,
}

#[wasm_bindgen]
impl QueryEngine {
    /// Parse a validated datamodel and configuration to allow connecting later on.
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: ConstructorOptions,
        callback: JsFunction,
        adapter: JsObject,
    ) -> Result<QueryEngine, wasm_bindgen::JsError> {
        let log_callback = LogCallback(callback);

        let ConstructorOptions {
            datamodel,
            log_level,
            log_queries,
        } = options;

        // Note: if we used `psl::validate`, we'd add ~1MB to the Wasm artifact (before gzip).
        let schema = psl::parse_without_validation(datamodel.into(), CONNECTOR_REGISTRY);

        let js_queryable = Arc::new(driver_adapters::from_js(adapter));

        let enable_tracing = schema
            .configuration
            .preview_features()
            .contains(PreviewFeature::Tracing);

        let engine_protocol = EngineProtocol::Json;

        let builder = EngineBuilder {
            schema: Arc::new(schema),
            engine_protocol,
        };

        let log_level = log_level.parse::<LevelFilter>().unwrap_or(Level::INFO.into());
        let logger = Logger::new(log_queries, log_level, log_callback, enable_tracing);

        Ok(Self {
            inner: RwLock::new(Inner::Builder(builder)),
            adapter: js_queryable,
            logger,
        })
    }

    /// Connect to the database, allow queries to be run.
    #[wasm_bindgen]
    pub async fn connect(&self, trace: String, request_id: String) -> Result<(), wasm_bindgen::JsError> {
        let dispatcher = self.logger.dispatcher();
        let exporter = self.logger.exporter();

        async {
            let span = tracing::info_span!(
                "prisma:engine:connect",
                user_facing = true,
                request_id = tracing::field::Empty,
            );
            start_trace(&request_id, &trace, &span, &exporter).await?;

            let mut inner = self.inner.write().await;
            let builder = inner.as_builder()?;

            let preview_features = builder.schema.configuration.preview_features();
            let arced_schema = Arc::clone(&builder.schema);

            let engine = async move {
                let executor = load_executor(
                    ConnectorKind::Js {
                        adapter: Arc::clone(&self.adapter),
                        _phantom: PhantomData,
                    },
                    preview_features,
                )
                .await?;
                let connector = executor.primary_connector();

                let conn_span = tracing::info_span!(
                    "prisma:engine:connection",
                    user_facing = true,
                    "db.system" = connector.name(),
                );

                let conn = connector.get_connection().instrument(conn_span).await?;
                let db_version = conn.version().await;

                let query_schema_span = tracing::info_span!("prisma:engine:schema");

                let query_schema = query_schema_span
                    .in_scope(|| schema::build(arced_schema, true))
                    .with_db_version_supports_join_strategy(
                        relation_load_strategy::db_version_supports_joins_strategy(db_version)?,
                    );

                Ok(ConnectedEngine {
                    schema: builder.schema.clone(),
                    query_schema: Arc::new(query_schema),
                    executor,
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
    pub async fn disconnect(&self, trace: String, request_id: String) -> Result<(), wasm_bindgen::JsError> {
        let dispatcher = self.logger.dispatcher();
        let exporter = self.logger.exporter();

        async {
            let span = tracing::info_span!(
                "prisma:engine:disconnect",
                user_facing = true,
                request_id = tracing::field::Empty,
            );
            start_trace(&request_id, &trace, &span, &exporter).await?;

            async {
                let mut inner = self.inner.write().await;
                let engine = inner.as_engine()?;

                let builder = EngineBuilder {
                    schema: engine.schema.clone(),
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
        request_id: String,
    ) -> Result<String, wasm_bindgen::JsError> {
        let dispatcher = self.logger.dispatcher();
        let exporter = self.logger.exporter();

        async {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let query = RequestBody::try_from_str(&body, engine.engine_protocol())?;

            async move {
                let span = tracing::info_span!(
                    "prisma:engine:query",
                    user_facing = true,
                    request_id = tracing::field::Empty,
                );

                let traceparent = start_trace(&request_id, &trace, &span, &exporter).await?;

                let handler = RequestHandler::new(engine.executor(), engine.query_schema(), engine.engine_protocol());
                let response = handler
                    .handle(query, tx_id.map(TxId::from), traceparent)
                    .instrument(span)
                    .await;

                let serde_span = tracing::info_span!("prisma:engine:response_json_serialization", user_facing = true);
                Ok(serde_span.in_scope(|| serde_json::to_string(&response))?)
            }
            .await
        }
        .with_subscriber(dispatcher)
        .await
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    #[wasm_bindgen(js_name = startTransaction)]
    pub async fn start_transaction(
        &self,
        input: String,
        trace: String,
        request_id: String,
    ) -> Result<String, wasm_bindgen::JsError> {
        let inner = self.inner.read().await;
        let engine = inner.as_engine()?;
        let dispatcher = self.logger.dispatcher();
        let exporter = self.logger.exporter();

        async move {
            let span = tracing::info_span!(
                "prisma:engine:start_transaction",
                user_facing = true,
                request_id = tracing::field::Empty,
            );
            start_trace(&request_id, &trace, &span, &exporter).await?;

            let tx_opts: TransactionOptions = serde_json::from_str(&input)?;
            match engine
                .executor()
                .start_tx(engine.query_schema().clone(), engine.engine_protocol(), tx_opts)
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
    pub async fn commit_transaction(
        &self,
        tx_id: String,
        trace: String,
        request_id: String,
    ) -> Result<String, wasm_bindgen::JsError> {
        let inner = self.inner.read().await;
        let engine = inner.as_engine()?;

        let dispatcher = self.logger.dispatcher();
        let exporter = self.logger.exporter();

        async move {
            let span = tracing::info_span!(
                "prisma:engine:commit_transaction",
                user_facing = true,
                request_id = tracing::field::Empty
            );
            start_trace(&request_id, &trace, &span, &exporter).await?;

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
    pub async fn rollback_transaction(
        &self,
        tx_id: String,
        trace: String,
        request_id: String,
    ) -> Result<String, wasm_bindgen::JsError> {
        let inner = self.inner.read().await;
        let engine = inner.as_engine()?;

        let dispatcher = self.logger.dispatcher();
        let exporter = self.logger.exporter();

        async move {
            let span = tracing::info_span!(
                "prisma:engine:rollback_transaction",
                user_facing = true,
                request_id = tracing::field::Empty,
            );
            start_trace(&request_id, &trace, &span, &exporter).await?;

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

    /// Fetch the spans associated with a [`RequestId`]
    #[wasm_bindgen]
    pub async fn trace(&self, request_id: String) -> Result<Option<String>, wasm_bindgen::JsError> {
        let dispatcher = self.logger.dispatcher();
        let exporter = self.logger.exporter();

        async {
            let request_id = request_id
                .parse()
                .map_err(|_| ApiError::Decode("invalid request id".into()))?;

            Ok(exporter
                .stop_capturing(request_id)
                .await
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?)
        }
        .with_subscriber(dispatcher)
        .await
    }
}
