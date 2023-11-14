mod connection;
#[cfg(feature = "driver-adapters")]
mod js;
mod transaction;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native {
    pub(crate) mod mssql;
    pub(crate) mod mysql;
    pub(crate) mod postgresql;
    pub(crate) mod sqlite;
}

pub(crate) mod operations;

use async_trait::async_trait;
use connector_interface::{error::ConnectorError, Connector};

#[cfg(feature = "driver-adapters")]
pub use js::*;

#[cfg(not(target_arch = "wasm32"))]
pub use native::{mssql::*, mysql::*, postgresql::*, sqlite::*};

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
    async fn from_source(
        source: &psl::Datasource,
        url: &str,
        features: psl::PreviewFeatures,
    ) -> connector_interface::Result<Self>
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
