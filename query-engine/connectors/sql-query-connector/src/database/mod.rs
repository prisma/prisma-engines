mod connection;
mod mssql;
mod mysql;
mod postgresql;
mod sqlite;
mod transaction;

pub(crate) mod operations;

use async_trait::async_trait;
use connector_interface::{error::ConnectorError, Connector};
use datamodel::Datasource;

pub use mssql::*;
pub use mysql::*;
pub use postgresql::*;
pub use sqlite::*;

#[async_trait]
pub trait FromSource {
    /// Instantiate a query connector from a Datasource.
    ///
    /// The resolved url is passed distinctly from the datasource for two
    /// reasons:
    ///
    /// 1. Extracting the final url from the datasource involves resolving env
    ///    vars and validating, which can fail with a schema parser error. We
    ///    want to handle this as early as possible and in a single place.
    ///
    /// 2. The url may be modified with the config dir, in the case of Node-API.
    async fn from_source(source: &Datasource, url: &str) -> connector_interface::Result<Self>
    where
        Self: Connector + Sized;
}

async fn catch<O>(
    connection_info: quaint::prelude::ConnectionInfo,
    fut: impl std::future::Future<Output = Result<O, crate::SqlError>>,
) -> Result<O, ConnectorError> {
    match fut.await {
        Ok(o) => Ok(o),
        Err(err) => Err(err.into_connector_error(&connection_info)),
    }
}
