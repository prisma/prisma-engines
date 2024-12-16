#![allow(unused_imports)]

use psl::{builtin_connectors::*, Datasource, PreviewFeatures};
use quaint::connector::ExternalConnector;
use query_core::{executor::InterpretingExecutor, Connector, QueryExecutor};
use sql_query_connector::*;
use std::collections::HashMap;
use std::env;
use std::marker::PhantomData;
use std::sync::Arc;
use url::Url;

pub enum ConnectorKind<'a> {
    #[cfg(native)]
    Rust { url: String, datasource: &'a Datasource },
    Js {
        adapter: Arc<dyn ExternalConnector>,
        _phantom: PhantomData<&'a ()>, // required for WASM target, where JS is the only variant and lifetime gets unused
    },
}

/// Loads a query executor based on the parsed Prisma schema (datasource).
pub async fn load(
    connector_kind: ConnectorKind<'_>,
    features: PreviewFeatures,
    #[allow(unused_variables)] tracing_enabled: bool,
) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync + 'static>> {
    match connector_kind {
        #[cfg(not(feature = "driver-adapters"))]
        ConnectorKind::Js { .. } => {
            panic!("Driver adapters are not enabled, but connector mode is set to JS");
        }

        #[cfg(feature = "driver-adapters")]
        ConnectorKind::Js { adapter, _phantom } => driver_adapter(adapter, features).await,

        #[cfg(native)]
        ConnectorKind::Rust { url, datasource } => {
            if let Ok(value) = env::var("PRISMA_DISABLE_QUAINT_EXECUTORS") {
                let disable = value.to_uppercase();
                if disable == "TRUE" || disable == "1" {
                    panic!("Quaint executors are disabled, as per env var PRISMA_DISABLE_QUAINT_EXECUTORS.");
                }
            }

            match datasource.active_provider {
                #[cfg(feature = "sqlite-native")]
                p if SQLITE.is_provider(p) => native::sqlite(datasource, &url, features, tracing_enabled).await,
                #[cfg(feature = "mysql-native")]
                p if MYSQL.is_provider(p) => native::mysql(datasource, &url, features, tracing_enabled).await,
                #[cfg(feature = "postgresql-native")]
                p if POSTGRES.is_provider(p) => native::postgres(datasource, &url, features, tracing_enabled).await,
                #[cfg(feature = "mssql-native")]
                p if MSSQL.is_provider(p) => native::mssql(datasource, &url, features, tracing_enabled).await,
                #[cfg(feature = "cockroachdb-native")]
                p if COCKROACH.is_provider(p) => native::postgres(datasource, &url, features, tracing_enabled).await,
                #[cfg(feature = "mongodb")]
                p if MONGODB.is_provider(p) => native::mongodb(datasource, &url, features, tracing_enabled).await,

                x => Err(query_core::CoreError::ConfigurationError(format!(
                    "Unsupported connector type: {x}"
                ))),
            }
        }
    }
}

#[cfg(feature = "driver-adapters")]
async fn driver_adapter(
    driver_adapter: Arc<dyn ExternalConnector>,
    features: PreviewFeatures,
) -> Result<Box<dyn QueryExecutor + Send + Sync>, query_core::CoreError> {
    use quaint::connector::ExternalConnector;

    let js = Js::new(driver_adapter, features).await?;
    Ok(executor_for(js, false))
}

#[cfg(native)]
mod native {
    use super::*;
    use tracing::trace;

    #[cfg(feature = "sqlite-native")]
    pub(crate) async fn sqlite(
        source: &Datasource,
        url: &str,
        features: PreviewFeatures,
        tracing_enabled: bool,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        trace!("Loading SQLite query connector...");
        let sqlite = Sqlite::from_source(source, url, features, tracing_enabled).await?;
        trace!("Loaded SQLite query connector.");
        Ok(executor_for(sqlite, false))
    }

    #[cfg(feature = "postgresql-native")]
    pub(crate) async fn postgres(
        source: &Datasource,
        url: &str,
        features: PreviewFeatures,
        tracing_enabled: bool,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        trace!("Loading Postgres query connector...");
        let database_str = url;
        let psql = PostgreSql::from_source(source, url, features, tracing_enabled).await?;

        let url = Url::parse(database_str).map_err(|err| {
            query_core::CoreError::ConfigurationError(format!("Error parsing connection string: {err}"))
        })?;
        let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

        let force_transactions = params
            .get("pgbouncer")
            .and_then(|flag| flag.parse().ok())
            .unwrap_or(false);
        trace!("Loaded Postgres query connector.");
        Ok(executor_for(psql, force_transactions))
    }

    #[cfg(feature = "mysql-native")]
    pub(crate) async fn mysql(
        source: &Datasource,
        url: &str,
        features: PreviewFeatures,
        tracing_enabled: bool,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        let mysql = Mysql::from_source(source, url, features, tracing_enabled).await?;
        trace!("Loaded MySQL query connector.");
        Ok(executor_for(mysql, false))
    }

    #[cfg(feature = "mssql-native")]
    pub(crate) async fn mssql(
        source: &Datasource,
        url: &str,
        features: PreviewFeatures,
        tracing_enabled: bool,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        trace!("Loading SQL Server query connector...");
        let mssql = Mssql::from_source(source, url, features, tracing_enabled).await?;
        trace!("Loaded SQL Server query connector.");
        Ok(executor_for(mssql, false))
    }

    #[cfg(feature = "mongodb")]
    pub(crate) async fn mongodb(
        source: &Datasource,
        url: &str,
        _features: PreviewFeatures,
        _tracing_enabled: bool,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        use mongodb_query_connector::MongoDb;

        trace!("Loading MongoDB query connector...");
        let mongo = MongoDb::new(source, url).await?;
        trace!("Loaded MongoDB query connector.");
        Ok(executor_for(mongo, false))
    }
}

fn executor_for<T>(connector: T, force_transactions: bool) -> Box<dyn QueryExecutor + Send + Sync>
where
    T: Connector + Send + Sync + 'static,
{
    Box::new(InterpretingExecutor::new(connector, force_transactions))
}
