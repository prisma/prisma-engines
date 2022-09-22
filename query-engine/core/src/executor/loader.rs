use super::{interpreting_executor::InterpretingExecutor, QueryExecutor};
use crate::CoreError;
use connection_string::JdbcString;
#[cfg(feature = "sql")]
use connector::Connector;
#[cfg(feature = "mongodb")]
use mongodb_client::MongoConnectionString;
use psl::{builtin_connectors::*, common::preview_features::PreviewFeature, Datasource};
#[cfg(feature = "sql")]
use sql_connector::*;
use std::collections::HashMap;
use std::str::FromStr;
use url::Url;

#[cfg(feature = "mongodb")]
use mongodb_connector::MongoDb;

const DEFAULT_SQLITE_DB_NAME: &str = "main";

/// Loads a query executor based on the parsed Prisma schema (datasource).
pub async fn load(
    source: &Datasource,
    features: &[PreviewFeature],
    url: &str,
) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    match source.active_provider {
        #[cfg(feature = "sql")]
        p if SQLITE.is_provider(p) => sqlite(source, url, features).await,
        #[cfg(feature = "sql")]
        p if MYSQL.is_provider(p) => mysql(source, url, features).await,
        #[cfg(feature = "sql")]
        p if POSTGRES.is_provider(p) => postgres(source, url, features).await,
        #[cfg(feature = "sql")]
        p if MSSQL.is_provider(p) => mssql(source, url, features).await,
        #[cfg(feature = "sql")]
        p if COCKROACH.is_provider(p) => postgres(source, url, features).await,

        #[cfg(feature = "mongodb")]
        p if MONGODB.is_provider(p) => mongodb(source, url, features).await,

        x => Err(CoreError::ConfigurationError(format!(
            "Unsupported connector type: {}",
            x
        ))),
    }
}

pub fn db_name(source: &Datasource, url: &str) -> crate::Result<String> {
    match source.active_provider {
        p if SQLITE.is_provider(p) => Ok(DEFAULT_SQLITE_DB_NAME.to_string()),
        p if MYSQL.is_provider(p) => {
            let url = Url::parse(url)?;
            let err_str = "No database found in connection string";

            let mut db_name = url
                .path_segments()
                .ok_or_else(|| CoreError::ConfigurationError(err_str.into()))?;

            let db_name = db_name.next().expect(err_str).to_owned();

            Ok(db_name)
        }
        p if POSTGRES.is_provider(p) | COCKROACH.is_provider(p) => {
            let url = Url::parse(url)?;
            let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

            let db_name = params
                .get("schema")
                .map(ToString::to_string)
                .unwrap_or_else(|| String::from("public"));

            Ok(db_name)
        }
        p if MSSQL.is_provider(p) => {
            let mut conn = JdbcString::from_str(&format!("jdbc:{}", url))?;
            let db_name = conn
                .properties_mut()
                .remove("schema")
                .unwrap_or_else(|| String::from("dbo"));

            Ok(db_name)
        }
        #[cfg(feature = "mongodb")]
        p if MONGODB.is_provider(p) => {
            let url: MongoConnectionString = url.parse().map_err(|e: mongodb_client::Error| match &e.kind {
                mongodb_client::ErrorKind::InvalidArgument { message } => {
                    CoreError::ConfigurationError(format!("Error parsing connection string: {}", message))
                }
                _ => {
                    let kind = connector::error::ErrorKind::ConnectionError(e.into());
                    CoreError::ConnectorError(connector::error::ConnectorError::from_kind(kind))
                }
            })?;

            Ok(url.database)
        }
        x => Err(CoreError::ConfigurationError(format!(
            "Unsupported connector type: {}",
            x
        ))),
    }
}

#[cfg(feature = "sql")]
async fn sqlite(
    source: &Datasource,
    url: &str,
    features: &[PreviewFeature],
) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading SQLite query connector...");

    let sqlite = Sqlite::from_source(source, url, features).await?;

    let db_name = db_name(source, url)?;

    trace!("Loaded SQLite query connector.");
    Ok((db_name, sql_executor(sqlite, false)))
}

#[cfg(feature = "sql")]
async fn postgres(
    source: &Datasource,
    url: &str,
    features: &[PreviewFeature],
) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading Postgres query connector...");

    let database_str = url;
    let psql = PostgreSql::from_source(source, url, features).await?;

    let url = Url::parse(database_str)?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let force_transactions = params
        .get("pgbouncer")
        .and_then(|flag| flag.parse().ok())
        .unwrap_or(false);

    let db_name = db_name(source, database_str)?;

    trace!("Loaded Postgres query connector.");
    Ok((db_name, sql_executor(psql, force_transactions)))
}

#[cfg(feature = "sql")]
async fn mysql(
    source: &Datasource,
    url: &str,
    features: &[PreviewFeature],
) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading MySQL query connector...");

    let mysql = Mysql::from_source(source, url, features).await?;

    let db_name = db_name(source, url)?;

    trace!("Loaded MySQL query connector.");
    Ok((db_name, sql_executor(mysql, false)))
}

#[cfg(feature = "sql")]
async fn mssql(
    source: &Datasource,
    url: &str,
    features: &[PreviewFeature],
) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading SQL Server query connector...");

    let mssql = Mssql::from_source(source, url, features).await?;

    let db_name = db_name(source, url)?;

    trace!("Loaded SQL Server query connector.");
    Ok((db_name, sql_executor(mssql, false)))
}

#[cfg(feature = "sql")]
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
    _features: &[PreviewFeature],
) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading MongoDB query connector...");

    let mongo = MongoDb::new(source, url).await?;
    let db_name = db_name(source, url)?;

    trace!("Loaded MongoDB query connector.");
    Ok((db_name.to_owned(), Box::new(InterpretingExecutor::new(mongo, false))))
}
