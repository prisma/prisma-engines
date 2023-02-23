mod binary;
mod direct;
mod json_adapter;
mod node_api;

pub use binary::*;
pub use direct::*;
pub use json_adapter::*;
pub use node_api::*;

use query_core::{protocol::EngineProtocol, schema::QuerySchemaRef, TxId};
use query_engine_metrics::MetricRegistry;

use crate::{ConnectorTag, ConnectorVersion, QueryResult, TestError, TestLogCapture, TestResult, ENGINE_PROTOCOL};
use colored::*;

pub type TxResult = Result<(), user_facing_errors::Error>;

#[async_trait::async_trait]
pub trait RunnerInterface: Sized {
    /// Initializes the runner.
    async fn load(datamodel: String, connector_tag: ConnectorTag, metrics: MetricRegistry) -> TestResult<Self>;

    /// Queries the engine using GraphQL.
    /// If 'protocol' is set to JSON, then the GQL query will be translated to JSON before being sent to the QueryEngine.
    async fn query_graphql(&self, query: String, protocol: &EngineProtocol) -> TestResult<QueryResult>;

    /// Queries the engine using JSON.
    async fn query_json(&self, query: String) -> TestResult<QueryResult>;

    /// Queries the engine with a batch.
    async fn batch(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> TestResult<QueryResult>;

    /// Execute a raw query on the underlying connected database.
    async fn raw_execute(&self, query: String) -> TestResult<()>;

    /// start a transaction for a batch run
    async fn start_tx(
        &self,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
        isolation_level: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> TestResult<TxId>;

    /// commit transaction
    async fn commit_tx(&self, tx_id: TxId) -> TestResult<TxResult>;

    /// rollback transaction
    async fn rollback_tx(&self, tx_id: TxId) -> TestResult<TxResult>;

    /// The connector tag used to load this runner.
    fn connector(&self) -> &ConnectorTag;

    /// Instructs this runner to use a specific ITX ID for queries.
    fn set_active_tx(&mut self, tx_id: TxId);

    /// Clears ITX ID for queries.
    fn clear_active_tx(&mut self);

    fn get_metrics(&self) -> MetricRegistry;

    /// The query schema used for the test.
    fn query_schema(&self) -> &QuerySchemaRef;
}

enum RunnerType {
    /// Using the QE crate directly for queries.
    Direct(DirectRunner),

    /// Using a NodeJS runner.
    NodeApi(NodeApiRunner),

    /// Using the HTTP bridge
    Binary(BinaryRunner),
}

pub struct Runner {
    log_capture: TestLogCapture,
    inner: RunnerType,
    protocol: EngineProtocol,
}

impl Runner {
    pub async fn load(
        ident: &str,
        datamodel: String,
        connector_tag: ConnectorTag,
        metrics: MetricRegistry,
        log_capture: TestLogCapture,
    ) -> TestResult<Self> {
        let inner = match ident {
            "direct" => Self::direct(datamodel, connector_tag, metrics).await,
            "node-api" => Ok(RunnerType::NodeApi(NodeApiRunner {})),
            "binary" => Self::binary(datamodel, connector_tag, metrics).await,
            unknown => Err(TestError::parse_error(format!("Unknown test runner '{unknown}'"))),
        }?;
        let protocol = EngineProtocol::from(&ENGINE_PROTOCOL.to_string());

        Ok(Self {
            log_capture,
            inner,
            protocol,
        })
    }

    pub async fn query<T>(&self, gql_query: T) -> TestResult<QueryResult>
    where
        T: Into<String>,
    {
        let gql_query = gql_query.into();

        tracing::debug!("Querying: {}", gql_query.clone().green());

        let response = match &self.inner {
            RunnerType::Direct(r) => r.query_graphql(gql_query, self.protocol()).await,
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.query_graphql(gql_query, self.protocol()).await,
        }?;

        if response.failed() {
            tracing::debug!("Response: {}", response.to_string().red());
        } else {
            tracing::debug!("Response: {}", response.to_string().green());
        }

        Ok(response)
    }

    pub async fn query_json<T>(&self, json_query: T) -> TestResult<QueryResult>
    where
        T: Into<String>,
    {
        let json_query = json_query.into();

        tracing::debug!("Querying: {}", json_query.clone().green());

        let response = match &self.inner {
            RunnerType::Direct(r) => r.query_json(json_query).await,
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.query_json(json_query).await,
        }?;

        if response.failed() {
            tracing::debug!("Response: {}", response.to_string().red());
        } else {
            tracing::debug!("Response: {}", response.to_string().green());
        }

        Ok(response)
    }

    pub async fn raw_execute<T>(&self, sql: T) -> TestResult<()>
    where
        T: Into<String>,
    {
        let sql = sql.into();
        tracing::debug!("Raw execute: {}", sql.clone().green());

        match &self.inner {
            RunnerType::Direct(r) => r.raw_execute(sql).await,
            RunnerType::Binary(r) => r.raw_execute(sql).await,
            RunnerType::NodeApi(_) => todo!(),
        }
    }

    pub async fn start_tx(
        &self,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
        isolation_level: Option<String>,
    ) -> TestResult<TxId> {
        match &self.inner {
            RunnerType::Direct(r) => {
                r.start_tx(max_acquisition_millis, valid_for_millis, isolation_level, self.protocol)
                    .await
            }
            RunnerType::Binary(r) => {
                r.start_tx(max_acquisition_millis, valid_for_millis, isolation_level, self.protocol)
                    .await
            }
            RunnerType::NodeApi(_) => todo!(),
        }
    }

    pub async fn commit_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        match &self.inner {
            RunnerType::Direct(r) => r.commit_tx(tx_id).await,
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.commit_tx(tx_id).await,
        }
    }

    pub async fn rollback_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        match &self.inner {
            RunnerType::Direct(r) => r.rollback_tx(tx_id).await,
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.rollback_tx(tx_id).await,
        }
    }

    pub async fn batch(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
    ) -> TestResult<QueryResult> {
        match &self.inner {
            RunnerType::Direct(r) => r.batch(queries, transaction, isolation_level, self.protocol).await,
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.batch(queries, transaction, isolation_level, self.protocol).await,
        }
    }

    async fn direct(datamodel: String, connector_tag: ConnectorTag, metrics: MetricRegistry) -> TestResult<RunnerType> {
        let runner = DirectRunner::load(datamodel, connector_tag, metrics).await?;

        Ok(RunnerType::Direct(runner))
    }

    async fn binary(datamodel: String, connector_tag: ConnectorTag, metrics: MetricRegistry) -> TestResult<RunnerType> {
        let runner = BinaryRunner::load(datamodel, connector_tag, metrics).await?;

        Ok(RunnerType::Binary(runner))
    }

    pub fn connector(&self) -> &ConnectorTag {
        match &self.inner {
            RunnerType::Direct(r) => r.connector(),
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.connector(),
        }
    }

    pub fn connector_version(&self) -> ConnectorVersion {
        match &self.inner {
            RunnerType::Direct(r) => ConnectorVersion::from(r.connector()),
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => ConnectorVersion::from(r.connector()),
        }
    }

    pub fn set_active_tx(&mut self, tx_id: TxId) {
        match &mut self.inner {
            RunnerType::Direct(r) => r.set_active_tx(tx_id),
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.set_active_tx(tx_id),
        }
    }

    pub fn clear_active_tx(&mut self) {
        match &mut self.inner {
            RunnerType::Direct(r) => r.clear_active_tx(),
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.clear_active_tx(),
        }
    }

    pub fn get_metrics(&self) -> MetricRegistry {
        match &self.inner {
            RunnerType::Direct(r) => r.get_metrics(),
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.get_metrics(),
        }
    }

    pub async fn get_logs(&mut self) -> Vec<String> {
        self.log_capture.get_logs().await
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        match &self.inner {
            RunnerType::Direct(r) => r.query_schema(),
            RunnerType::NodeApi(_) => todo!(),
            RunnerType::Binary(r) => r.query_schema(),
        }
    }

    pub fn protocol(&self) -> &EngineProtocol {
        &self.protocol
    }
}
