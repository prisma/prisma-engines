mod json_adapter;
mod transaction;

pub use json_adapter::*;
pub use transaction::*;

use serde::{Deserialize, Serialize};

use crate::{
    ConnectorTag, ConnectorVersion, ENGINE_PROTOCOL, QueryResult, RenderedDatamodel, TestError, TestLogCapture,
    TestResult, executor_process_request,
};
use colored::Colorize;
use query_core::{
    protocol::EngineProtocol,
    schema::{self, QuerySchemaRef},
};
use request_handlers::{BatchTransactionOption, JsonBatchQuery, JsonBody, JsonSingleQuery};
use serde_json::json;
use std::{
    fmt::Display,
    sync::{Arc, atomic::AtomicUsize},
};

pub type TxResult = Result<(), user_facing_errors::Error>;

#[derive(Deserialize, Debug)]
struct Empty {}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum TransactionEndResponse {
    Error(user_facing_errors::Error),
    Ok(Empty),
}

impl From<TransactionEndResponse> for TxResult {
    fn from(value: TransactionEndResponse) -> Self {
        match value {
            TransactionEndResponse::Ok(_) => Ok(()),
            TransactionEndResponse::Error(error) => Err(error),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum StartTransactionResponse {
    Ok { id: String },
    Error(user_facing_errors::Error),
}

/// [`ExternalExecutor::schema_id`] represents the index of the schema used for the test suite to
/// execute queries against. When the suite starts, a message with the schema and the id is sent to
/// the external process, which will create a new instance of the library engine configured to
/// access that schema.
///
/// Everytime a query is sent to the external process, it's provided the id of the schema, so the
/// process knows how to associate the query to the instance of the library engine that will dispatch
/// it.
#[derive(Copy, Clone)]
pub struct ExternalExecutor {
    schema_id: usize,
}

/// [`ExternalExecutorInitializer`] is responsible for initialising a test session for the external process.
/// The initialisation can happen with or without a migration script, and is performed by submitting the
/// "initializeSchema" JSON-RPC request.
/// [`ExternalExecutorInitializer::schema_id`] is the schema id of the parent [`ExternalExecutor`].
/// [`ExternalExecutorInitializer::url`] and [`ExternalExecutorInitializer::schema`] are the context
/// necessary for the "initializeSchema" JSON-RPC request.
/// The usage of `&'a str` is to avoid problems with `String` not implementing the `Copy` trait.
struct ExternalExecutorInitializer<'a> {
    schema_id: usize,
    url: &'a str,
    schema: &'a str,
}

impl<'a> qe_setup::ExternalInitializer<'a> for ExternalExecutorInitializer<'a> {
    async fn init_with_migration(
        &self,
        migration_script: String,
    ) -> Result<qe_setup::InitResult, Box<dyn std::error::Error + Send + Sync>> {
        let migration_script = Some(migration_script);
        let init_result = executor_process_request("initializeSchema", json!({ "schemaId": self.schema_id, "schema": self.schema, "url": self.url, "migrationScript": migration_script })).await?;
        Ok(init_result)
    }

    async fn init(&self) -> Result<qe_setup::InitResult, Box<dyn std::error::Error + Send + Sync>> {
        let init_result = executor_process_request(
            "initializeSchema",
            json!({ "schemaId": self.schema_id, "schema": self.schema, "url": self.url }),
        )
        .await?;
        Ok(init_result)
    }

    fn url(&self) -> &'a str {
        self.url
    }

    fn datamodel(&self) -> &'a str {
        self.schema
    }
}

impl ExternalExecutor {
    /// Request a new schema id to be used for the external process.
    /// This operation wraps around on overflow.
    fn external_schema_id() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    fn new() -> Self {
        let schema_id = Self::external_schema_id();
        Self { schema_id }
    }

    /// Create a temporary initializer for external Driver Adapters.
    fn init<'a>(&self, datamodel: &'a str, url: &'a str) -> ExternalExecutorInitializer<'a> {
        ExternalExecutorInitializer {
            schema_id: self.schema_id,
            url,
            schema: datamodel,
        }
    }

    pub(self) async fn query<JsonQuery: Serialize>(
        &self,
        json_query: JsonQuery,
        current_tx_id: Option<&TxId>,
    ) -> Result<QueryResult, Box<dyn std::error::Error + Send + Sync>> {
        let response_str: String = executor_process_request(
            "query",
            json!({ "schemaId": self.schema_id, "query": json_query, "txId": current_tx_id.map(ToString::to_string) }),
        )
        .await?;
        let response: QueryResult = serde_json::from_str(&response_str).unwrap();
        Ok(response)
    }

    pub(self) async fn start_tx(
        &self,
        tx_opts: TransactionOptions,
    ) -> Result<StartTransactionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let response: StartTransactionResponse =
            executor_process_request("startTx", json!({ "schemaId": self.schema_id, "options": tx_opts })).await?;
        Ok(response)
    }

    pub(self) async fn commit_tx(
        &self,
        tx_id: TxId,
    ) -> Result<TransactionEndResponse, Box<dyn std::error::Error + Send + Sync>> {
        let response: TransactionEndResponse = executor_process_request(
            "commitTx",
            json!({ "schemaId": self.schema_id, "txId": tx_id.to_string() }),
        )
        .await?;
        Ok(response)
    }

    pub(self) async fn rollback_tx(
        &self,
        tx_id: TxId,
    ) -> Result<TransactionEndResponse, Box<dyn std::error::Error + Send + Sync>> {
        let response: TransactionEndResponse = executor_process_request(
            "rollbackTx",
            json!({ "schemaId": self.schema_id, "txId": tx_id.to_string() }),
        )
        .await?;
        Ok(response)
    }

    pub(crate) async fn get_logs(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let response: Vec<String> = executor_process_request("getLogs", json!({ "schemaId": self.schema_id })).await?;
        Ok(response)
    }
}

/// Direct engine runner.
pub struct Runner {
    executor: ExternalExecutor,
    query_schema: QuerySchemaRef,
    version: ConnectorVersion,
    connector_tag: ConnectorTag,
    connection_url: String,
    current_tx_id: Option<TxId>,
    protocol: EngineProtocol,
    log_capture: TestLogCapture,
    local_max_bind_values: Option<usize>,
}

impl Runner {
    pub(crate) fn schema_id(&self) -> usize {
        self.executor.schema_id
    }

    pub fn prisma_dml(&self) -> &str {
        self.query_schema.internal_data_model.schema.db.source_assert_single()
    }

    pub fn max_bind_values(&self) -> Option<usize> {
        self.local_max_bind_values
            .or_else(|| self.connector_version().max_bind_values())
    }

    pub async fn load(
        url: &str,
        datamodel: &RenderedDatamodel,
        db_schemas: &[&str],
        connector_version: ConnectorVersion,
        connector_tag: ConnectorTag,
        override_local_max_bind_values: Option<usize>,
        log_capture: TestLogCapture,
    ) -> TestResult<Self> {
        let protocol = EngineProtocol::from(&ENGINE_PROTOCOL.to_string());
        let schema = psl::parse_schema_without_extensions(&datamodel.schema).unwrap();

        let executor = ExternalExecutor::new();

        let external_initializer = executor.init(&datamodel.schema, url);

        let init_external_result = qe_setup::setup_external(
            crate::CONFIG.with_driver_adapter().adapter,
            external_initializer,
            db_schemas,
        )
        .await?;

        // If `override_local_max_bind_values` is provided, use that.
        // Otherwise, if the external process has provided an `init_result`, use `init_result.max_bind_values`.
        // Otherwise, use the connector's (Wasm-aware) default.
        //
        // Note: Use `override_local_max_bind_values` only for local testing purposes.
        // If a feature requires a specific `max_bind_values` value for a Driver Adapter, it should be set in the
        // TypeScript Driver Adapter implementation itself.
        let local_max_bind_values = match (override_local_max_bind_values, init_external_result) {
            (Some(override_max_bind_values), _) => Some(override_max_bind_values),
            (_, init_result) => init_result.max_bind_values,
        };

        let query_schema = schema::build(Arc::new(schema), true);

        // TODO: this currently doesn't work with driver adapters because we use the `connector_version` field
        // in them to duplicate the information about the driver adapter itself and not to represent the database
        // version. We should introduce separate test attributes for filtering by driver adapters (`only_adapters`/
        // `exclude_adapters`) and use `connector_version` for the actual database version like in legacy QE test
        // configs instead. Moreover, we should start testing multiple versions of the databases again, just now with
        // driver adapters.
        //
        // let query_schema = query_schema.with_db_version_supports_join_strategy(
        //     query_core::relation_load_strategy::db_version_supports_joins_strategy(
        //         crate::CONFIG.connector_version.clone(),
        //     )?,
        // );

        Ok(Self {
            version: connector_version,
            executor,
            query_schema: Arc::new(query_schema),
            connector_tag,
            connection_url: url.to_owned(),
            current_tx_id: None,
            protocol,
            log_capture,
            local_max_bind_values,
        })
    }

    pub async fn query(&self, query: impl Into<String>) -> TestResult<QueryResult> {
        self.query_with_params(self.current_tx_id.as_ref(), query).await
    }

    pub async fn query_in_tx(&self, tx_id: &TxId, query: impl Into<String>) -> TestResult<QueryResult> {
        self.query_with_params(Some(tx_id), query).await
    }

    async fn query_with_params<T>(&self, tx_id: Option<&TxId>, query: T) -> TestResult<QueryResult>
    where
        T: Into<String>,
    {
        match JsonRequest::from_graphql(&query.into(), self.query_schema()) {
            Ok(json_query) => {
                let mut response = self.executor.query(json_query, tx_id).await?;
                response.detag();
                Ok(response)
            }
            // Conversion from graphql to JSON might fail, and in that case we should consider the error
            // (a Handler error) as an error response.
            Err(TestError::RequestHandlerError(err)) => {
                let gql_err = request_handlers::GQLError::from_handler_error(err);
                let gql_res = request_handlers::GQLResponse::from(gql_err);
                let prisma_res = request_handlers::PrismaResponse::Single(gql_res);
                let mut response = QueryResult::from(prisma_res);
                response.detag();
                Ok(response)
            }
            Err(err) => Err(err),
        }
    }

    pub async fn query_json(&self, query: impl Display) -> TestResult<QueryResult> {
        let query = query.to_string();

        tracing::debug!("Querying: {}", query.clone().green());

        println!("{}", query.bright_green());
        let query: serde_json::Value = serde_json::from_str(&query).unwrap();

        let response = self.executor.query(query, self.current_tx_id.as_ref()).await?;

        Ok(response)
    }

    pub async fn raw_execute<T>(&self, query: T) -> TestResult<()>
    where
        T: Into<String>,
    {
        let query = query.into();
        tracing::debug!("Raw execute: {}", query.clone().green());

        self.connector_tag.raw_execute(&query, &self.connection_url).await?;

        Ok(())
    }

    pub async fn batch_json(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
    ) -> TestResult<crate::QueryResult> {
        tracing::debug!(
            ?isolation_level,
            transaction,
            "Batch query: {}",
            queries.join(", ").green()
        );

        let batch = queries
            .into_iter()
            .map(|query| serde_json::from_str::<JsonSingleQuery>(&query))
            .collect::<Result<Vec<_>, _>>()
            .expect("invalid json query");

        let transaction = transaction.then_some(BatchTransactionOption { isolation_level });
        let json_query = JsonBody::Batch(JsonBatchQuery { batch, transaction });
        let response: QueryResult = self.executor.query(json_query, self.current_tx_id.as_ref()).await?;

        Ok(response)
    }

    pub async fn batch(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
    ) -> TestResult<crate::QueryResult> {
        // Translate the GraphQL query to JSON
        let batch = queries
            .into_iter()
            .map(|query| JsonRequest::from_graphql(&query, self.query_schema()))
            .collect::<TestResult<Vec<_>>>()
            .unwrap();
        let transaction = match transaction {
            true => Some(BatchTransactionOption { isolation_level }),
            false => None,
        };
        let json_query = JsonBody::Batch(JsonBatchQuery { batch, transaction });
        let mut response: QueryResult = self.executor.query(json_query, self.current_tx_id.as_ref()).await?;
        response.detag();
        Ok(response)
    }

    pub async fn start_tx(
        &self,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
        isolation_level: Option<String>,
    ) -> TestResult<TxId> {
        let tx_opts = TransactionOptions::new(max_acquisition_millis, valid_for_millis, isolation_level);
        let response: StartTransactionResponse = self.executor.start_tx(tx_opts).await?;

        match response {
            StartTransactionResponse::Ok { id } => Ok(id.into()),
            StartTransactionResponse::Error(err) => {
                Err(crate::TestError::InteractiveTransactionError(err.message().into()))
            }
        }
    }

    pub async fn commit_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let response = self.executor.commit_tx(tx_id).await?;
        Ok(response.into())
    }

    pub async fn rollback_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let response = self.executor.rollback_tx(tx_id).await?;
        Ok(response.into())
    }

    pub fn connector(&self) -> &crate::ConnectorTag {
        &self.connector_tag
    }

    pub fn set_active_tx(&mut self, tx_id: TxId) {
        self.current_tx_id = Some(tx_id);
    }

    pub fn clear_active_tx(&mut self) {
        self.current_tx_id = None;
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }

    pub async fn get_logs(&mut self) -> Vec<String> {
        let mut logs = self.log_capture.get_logs().await;
        let mut external_logs = self.executor.get_logs().await.unwrap();
        logs.append(&mut external_logs);
        logs
    }

    pub async fn clear_logs(&mut self) {
        self.log_capture.clear_logs().await
    }

    pub fn connector_version(&self) -> &ConnectorVersion {
        &self.version
    }

    pub fn protocol(&self) -> EngineProtocol {
        self.protocol
    }
}
