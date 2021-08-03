mod binary;
mod direct;
mod node_api;

pub use binary::*;
pub use direct::*;
pub use node_api::*;
use query_core::{QueryExecutor, TxId};

use crate::{ConnectorTag, QueryResult, TestError, TestResult};
use colored::*;

#[async_trait::async_trait]
pub trait RunnerInterface: Sized {
    /// Initializes the runner.
    async fn load(datamodel: String, connector_tag: ConnectorTag) -> TestResult<Self>;

    /// Queries the engine.
    async fn query(&self, query: String) -> TestResult<QueryResult>;

    /// Queries the engine with a batch.
    async fn batch(&self, queries: Vec<String>, transaction: bool) -> TestResult<QueryResult>;

    /// The connector tag used to load this runner.
    fn connector(&self) -> &ConnectorTag;

    /// Exposes the underlying executor for testing.
    fn executor(&self) -> &dyn QueryExecutor;

    /// Instructs this runner to use a specific ITX ID for queries.
    fn set_active_tx(&mut self, tx_id: TxId);

    /// Clears ITX ID for queries.
    fn clear_active_tx(&mut self);
}

pub enum Runner {
    /// Using the QE crate directly for queries.
    Direct(DirectRunner),

    /// Using a NodeJS runner.
    NodeApi(NodeApiRunner),

    /// Using the HTTP bridge
    Binary(BinaryRunner),
}

impl Runner {
    pub async fn load(ident: &str, datamodel: String, connector_tag: ConnectorTag) -> TestResult<Self> {
        match ident {
            "direct" => Self::direct(datamodel, connector_tag).await,
            "node-api" => Ok(Self::NodeApi(NodeApiRunner {})),
            "binary" => Ok(Self::Binary(BinaryRunner {})),
            unknown => Err(TestError::parse_error(format!("Unknown test runner '{}'", unknown))),
        }
    }

    pub async fn query<T>(&self, gql_query: T) -> TestResult<QueryResult>
    where
        T: Into<String>,
    {
        let gql_query = gql_query.into();
        tracing::debug!("Querying: {}", gql_query.clone().green());

        let response = match self {
            Runner::Direct(r) => r.query(gql_query).await,
            Runner::NodeApi(_) => todo!(),
            Runner::Binary(_) => todo!(),
        }?;

        if response.failed() {
            tracing::debug!("Response: {}", response.to_string().red());
        } else {
            tracing::debug!("Response: {}", response.to_string().green());
        }

        Ok(response)
    }

    pub async fn batch(&self, queries: Vec<String>, transaction: bool) -> TestResult<QueryResult> {
        match self {
            Runner::Direct(r) => r.batch(queries, transaction).await,
            Runner::NodeApi(_) => todo!(),
            Runner::Binary(_) => todo!(),
        }
    }

    async fn direct(datamodel: String, connector_tag: ConnectorTag) -> TestResult<Self> {
        let runner = DirectRunner::load(datamodel, connector_tag).await?;

        Ok(Self::Direct(runner))
    }

    pub fn connector(&self) -> &ConnectorTag {
        match self {
            Runner::Direct(r) => r.connector(),
            Runner::NodeApi(_) => todo!(),
            Runner::Binary(_) => todo!(),
        }
    }

    pub fn executor(&self) -> &dyn QueryExecutor {
        match self {
            Runner::Direct(r) => r.executor(),
            Runner::NodeApi(_) => todo!(),
            Runner::Binary(_) => todo!(),
        }
    }

    pub fn set_active_tx(&mut self, tx_id: TxId) {
        match self {
            Runner::Direct(r) => r.set_active_tx(tx_id),
            Runner::NodeApi(_) => todo!(),
            Runner::Binary(_) => todo!(),
        }
    }

    pub fn clear_active_tx(&mut self) {
        match self {
            Runner::Direct(r) => r.clear_active_tx(),
            Runner::NodeApi(_) => todo!(),
            Runner::Binary(_) => todo!(),
        }
    }
}
