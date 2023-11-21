#![allow(unused_imports)]

use psl::{builtin_connectors::*, Datasource, PreviewFeatures};
use query_core::{executor::InterpretingExecutor, Connector, QueryExecutor};
use sql_query_connector::*;
use std::collections::HashMap;
use std::env;
use url::Url;

use super::ConnectorMode;

/// Loads a query executor based on the parsed Prisma schema (datasource).
pub async fn load(
    connector_mode: ConnectorMode,
    source: &Datasource,
    features: PreviewFeatures,
    url: &str,
) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync + 'static>> {
    match connector_mode {
        ConnectorMode::Js => {
            #[cfg(not(feature = "driver-adapters"))]
            panic!("Driver adapters are not enabled, but connector mode is set to JS");

            #[cfg(feature = "driver-adapters")]
            driver_adapter(source, url, features).await
        }

        #[cfg(feature = "native")]
        ConnectorMode::Rust => {
            if let Ok(value) = env::var("PRISMA_DISABLE_QUAINT_EXECUTORS") {
                let disable = value.to_uppercase();
                if disable == "TRUE" || disable == "1" {
                    panic!("Quaint executors are disabled, as per env var PRISMA_DISABLE_QUAINT_EXECUTORS.");
                }
            }

            match source.active_provider {
                p if SQLITE.is_provider(p) => native::sqlite(source, url, features).await,
                p if MYSQL.is_provider(p) => native::mysql(source, url, features).await,
                p if POSTGRES.is_provider(p) => native::postgres(source, url, features).await,
                p if MSSQL.is_provider(p) => native::mssql(source, url, features).await,
                p if COCKROACH.is_provider(p) => native::postgres(source, url, features).await,

                #[cfg(feature = "mongodb")]
                p if MONGODB.is_provider(p) => native::mongodb(source, url, features).await,

                x => Err(query_core::CoreError::ConfigurationError(format!(
                    "Unsupported connector type: {x}"
                ))),
            }
        }
    }
}

#[cfg(feature = "driver-adapters")]
async fn driver_adapter(
    source: &Datasource,
    url: &str,
    features: PreviewFeatures,
) -> Result<Box<dyn QueryExecutor + Send + Sync>, query_core::CoreError> {
    let js = Js::from_source(source, url, features).await?;
    Ok(executor_for(js, false))
}

#[cfg(feature = "native")]
mod native {
    use super::*;
    use tracing::trace;

    pub(crate) async fn sqlite(
        source: &Datasource,
        url: &str,
        features: PreviewFeatures,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        trace!("Loading SQLite query connector...");
        let sqlite = Sqlite::from_source(source, url, features).await?;
        trace!("Loaded SQLite query connector.");
        Ok(executor_for(sqlite, false))
    }

    pub(crate) async fn postgres(
        source: &Datasource,
        url: &str,
        features: PreviewFeatures,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        trace!("Loading Postgres query connector...");
        let database_str = url;
        let psql = PostgreSql::from_source(source, url, features).await?;

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

    pub(crate) async fn mysql(
        source: &Datasource,
        url: &str,
        features: PreviewFeatures,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        let mysql = Mysql::from_source(source, url, features).await?;
        trace!("Loaded MySQL query connector.");
        Ok(executor_for(mysql, false))
    }

    pub(crate) async fn mssql(
        source: &Datasource,
        url: &str,
        features: PreviewFeatures,
    ) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
        trace!("Loading SQL Server query connector...");
        let mssql = Mssql::from_source(source, url, features).await?;
        trace!("Loaded SQL Server query connector.");
        Ok(executor_for(mssql, false))
    }

    #[cfg(feature = "mongodb")]
    pub(crate) async fn mongodb(
        source: &Datasource,
        url: &str,
        _features: PreviewFeatures,
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
