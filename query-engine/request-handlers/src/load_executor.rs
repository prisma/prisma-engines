use nodejs_drivers::queryable::NodeJSQueryable;
use psl::{builtin_connectors::*, Datasource, PreviewFeatures};
use query_core::{executor::InterpretingExecutor, Connector, QueryExecutor};
use sql_query_connector::*;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::trace;
use url::Url;

#[cfg(feature = "mongodb")]
use mongodb_query_connector::MongoDb;

/// Loads a query executor based on the parsed Prisma schema (datasource).
pub async fn load(
    source: &Datasource,
    features: PreviewFeatures,
    url: &str,
    nodejs_queryable_lock: &RwLock<Option<NodeJSQueryable>>,
) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync + 'static>> {
    match source.active_provider {
        p if SQLITE.is_provider(p) => sqlite(source, url, features).await,
        p if MYSQL.is_provider(p) => mysql(source, url, features, nodejs_queryable_lock).await,
        p if POSTGRES.is_provider(p) => postgres(source, url, features).await,
        p if MSSQL.is_provider(p) => mssql(source, url, features).await,
        p if COCKROACH.is_provider(p) => postgres(source, url, features).await,

        #[cfg(feature = "mongodb")]
        p if MONGODB.is_provider(p) => mongodb(source, url, features).await,

        x => Err(query_core::CoreError::ConfigurationError(format!(
            "Unsupported connector type: {x}"
        ))),
    }
}

async fn sqlite(
    source: &Datasource,
    url: &str,
    features: PreviewFeatures,
) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
    trace!("Loading SQLite query connector...");
    let sqlite = Sqlite::from_source(source, url, features).await?;
    trace!("Loaded SQLite query connector.");
    Ok(sql_executor(sqlite, false))
}

async fn postgres(
    source: &Datasource,
    url: &str,
    features: PreviewFeatures,
) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
    trace!("Loading Postgres query connector...");

    let database_str = url;
    let psql = PostgreSql::from_source(source, url, features).await?;

    let url = Url::parse(database_str)
        .map_err(|err| query_core::CoreError::ConfigurationError(format!("Error parsing connection string: {err}")))?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let force_transactions = params
        .get("pgbouncer")
        .and_then(|flag| flag.parse().ok())
        .unwrap_or(false);

    trace!("Loaded Postgres query connector.");
    Ok(sql_executor(psql, force_transactions))
}

async fn mysql(
    source: &Datasource,
    url: &str,
    features: PreviewFeatures,
    nodejs_queryable_lock: &RwLock<Option<NodeJSQueryable>>,
) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
    trace!("Loading MySQL query connector...");

    if source.provider == "@prisma/mysql" {
        // write nodejs_queryable extracting the value of Option<NodeJSQueryable> and then take it:
        let mut nodejs_queryable_guard = nodejs_queryable_lock.write().await;
        let nodejs_queryable = (*nodejs_queryable_guard).take().unwrap();

        let mysql = Mysql::from_source_and_nodejs_driver(url, features, nodejs_queryable).await?;
        trace!("Loaded @prisma/mysql query connector.");
        return Ok(sql_executor(mysql, false));
    }

    let mysql = Mysql::from_source(source, url, features).await?;
    trace!("Loaded MySQL query connector.");
    Ok(sql_executor(mysql, false))
}

async fn mssql(
    source: &Datasource,
    url: &str,
    features: PreviewFeatures,
) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
    trace!("Loading SQL Server query connector...");
    let mssql = Mssql::from_source(source, url, features).await?;
    trace!("Loaded SQL Server query connector.");
    Ok(sql_executor(mssql, false))
}

fn sql_executor<T>(connector: T, force_transactions: bool) -> Box<dyn QueryExecutor + Send + Sync>
where
    T: Connector + Send + Sync + 'static,
{
    Box::new(InterpretingExecutor::new(connector, force_transactions))
}

#[cfg(feature = "mongodb")]
async fn mongodb(
    source: &Datasource,
    url: &str,
    _features: PreviewFeatures,
) -> query_core::Result<Box<dyn QueryExecutor + Send + Sync>> {
    trace!("Loading MongoDB query connector...");
    let mongo = MongoDb::new(source, url).await?;
    trace!("Loaded MongoDB query connector.");
    Ok(Box::new(InterpretingExecutor::new(mongo, false)))
}
