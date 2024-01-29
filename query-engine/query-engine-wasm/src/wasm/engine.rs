#![allow(dead_code)]
#![allow(unused_variables)]

use crate::{
    error::ApiError,
    logger::{LogCallback, Logger},
};
use driver_adapters::JsObject;
use js_sys::Function as JsFunction;
use psl::builtin_connectors::{MYSQL, POSTGRES, SQLITE};
use psl::ConnectorRegistry;
use quaint::connector::ExternalConnector;
use query_core::{
    protocol::EngineProtocol,
    schema::{self},
    telemetry, TransactionOptions, TxId,
};
use query_engine_common::engine::{map_known_error, ConnectedEngine, ConstructorOptions, EngineBuilder, Inner};
use request_handlers::ConnectorKind;
use request_handlers::{load_executor, RequestBody, RequestHandler};
use serde_json::json;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::RwLock;
use tracing::{field, instrument::WithSubscriber, Instrument, Level, Span};
use tracing_subscriber::filter::LevelFilter;
use wasm_bindgen::prelude::wasm_bindgen;

/// The main query engine used by JS
#[wasm_bindgen]
pub struct QueryEngine {
    inner: RwLock<String>,
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

        let js_queryable = Arc::new(driver_adapters::from_js(adapter));

        let log_level = log_level.parse::<LevelFilter>().unwrap_or(Level::INFO.into());
        let logger = Logger::new(log_queries, log_level, log_callback);

        Ok(Self {
            inner: RwLock::new(datamodel),
            adapter: js_queryable,
            logger,
        })
    }

    /// Connect to the database, allow queries to be run.
    #[wasm_bindgen]
    pub async fn connect(&self, trace: String) -> Result<(), wasm_bindgen::JsError> {
        todo!();
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    #[wasm_bindgen]
    pub async fn disconnect(&self, trace: String) -> Result<(), wasm_bindgen::JsError> {
        todo!();
    }

    /// If connected, sends a query to the core and returns the response.
    #[wasm_bindgen]
    pub async fn query(
        &self,
        body: String,
        trace: String,
        tx_id: Option<String>,
    ) -> Result<String, wasm_bindgen::JsError> {
        todo!();
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    #[wasm_bindgen(js_name = startTransaction)]
    pub async fn start_transaction(&self, input: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
        todo!();
    }

    /// If connected, attempts to commit a transaction with id `tx_id` in the core.
    #[wasm_bindgen(js_name = commitTransaction)]
    pub async fn commit_transaction(&self, tx_id: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
        todo!();
    }

    /// If connected, attempts to roll back a transaction with id `tx_id` in the core.
    #[wasm_bindgen(js_name = rollbackTransaction)]
    pub async fn rollback_transaction(&self, tx_id: String, trace: String) -> Result<String, wasm_bindgen::JsError> {
        todo!();
    }

    #[wasm_bindgen]
    pub async fn metrics(&self, json_options: String) -> Result<(), wasm_bindgen::JsError> {
        todo!();
    }
}
