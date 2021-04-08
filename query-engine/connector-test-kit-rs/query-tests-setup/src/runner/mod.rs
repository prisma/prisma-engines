mod binary;
mod direct;
mod napi;

pub use binary::*;
pub use direct::*;
pub use napi::*;

use crate::{QueryResult, TestError, TestResult};
use colored::*;

#[async_trait::async_trait]
pub trait RunnerInterface: Sized {
    async fn load(datamodel: String) -> TestResult<Self>;
    async fn query(&self, query: String) -> TestResult<QueryResult>;
    async fn batch(&self, queries: Vec<String>, transaction: bool) -> TestResult<QueryResult>;
}

pub enum Runner {
    /// Using the QE crate directly for queries.
    Direct(DirectRunner),

    /// Using a NodeJS runner.
    NApi(NApiRunner),

    /// Using the HTTP bridge
    Binary(BinaryRunner),
}

impl Runner {
    pub async fn load(ident: &str, datamodel: String) -> TestResult<Self> {
        match ident {
            "direct" => Self::direct(datamodel).await,
            "napi" => Ok(Self::NApi(NApiRunner {})),
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
            Runner::NApi(_) => todo!(),
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
            Runner::NApi(_) => todo!(),
            Runner::Binary(_) => todo!(),
        }
    }

    async fn direct(datamodel: String) -> TestResult<Self> {
        let runner = DirectRunner::load(datamodel).await?;

        Ok(Self::Direct(runner))
    }
}
