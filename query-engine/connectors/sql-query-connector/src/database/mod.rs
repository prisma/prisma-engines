mod connection;
#[cfg(feature = "driver-adapters")]
mod js;
mod transaction;

#[cfg(any(
    feature = "mssql-native",
    feature = "mysql-native",
    feature = "postgresql-native",
    feature = "sqlite-native"
))]
pub(crate) mod native {
    #[cfg(feature = "mssql")]
    pub(crate) mod mssql;
    #[cfg(feature = "mysql")]
    pub(crate) mod mysql;
    #[cfg(feature = "postgresql")]
    pub(crate) mod postgresql;
    #[cfg(feature = "sqlite")]
    pub(crate) mod sqlite;
}

pub(crate) mod operations;

use async_trait::async_trait;
use connector_interface::{error::ConnectorError, Connector};

#[cfg(feature = "driver-adapters")]
pub use js::*;

#[cfg(feature = "mssql-native")]
pub use native::mssql::*;

#[cfg(feature = "mysql-native")]
pub use native::mysql::*;

#[cfg(feature = "postgresql-native")]
pub use native::postgresql::*;

#[cfg(feature = "sqlite-native")]
pub use native::sqlite::*;

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

#[inline]
async fn catch<O>(
    connection_info: &quaint::prelude::ConnectionInfo,
    fut: impl std::future::Future<Output = Result<O, crate::SqlError>>,
) -> Result<O, ConnectorError> {
    fut.await.map_err(|err| err.into_connector_error(connection_info))
}
