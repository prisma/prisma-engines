mod binary;
mod direct;
mod node_api;

pub use binary::*;
pub use direct::*;
pub use node_api::*;

use crate::{ConnectorTag, QueryResult, TestError, TestResult};
use colored::*;

#[async_trait::async_trait]
pub trait RunnerInterface: Sized {
    async fn load(datamodel: String, connector_tag: ConnectorTag) -> TestResult<Self>;
    async fn query(&self, query: String) -> TestResult<QueryResult>;
    async fn batch(&self, queries: Vec<String>, transaction: bool) -> TestResult<QueryResult>;
    fn connector(&self) -> &ConnectorTag;
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

    pub async fn batch<T, S>(&self, gql_queries: T, transaction: bool) -> TestResult<QueryResult>
    where
        T: Iterator<Item = S>,
        S: Into<String>,
    {
        match self {
            Runner::Direct(r) => r.batch(gql_queries.map(Into::into).collect(), transaction).await,
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
}
