use crate::{PrismaError, PrismaResult};
use connector::Connector;

use datamodel::{
    configuration::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    Source,
};
use query_core::executor::{InterpretingExecutor, QueryExecutor};
use std::{collections::HashMap, path::PathBuf};
use url::Url;

#[cfg(feature = "sql")]
use sql_connector::*;

#[cfg(all(feature = "sql", feature = "mssql"))]
use datamodel::configuration::MSSQL_SOURCE_NAME;

pub async fn load(
    source: &(dyn Source + Send + Sync),
) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    match source.connector_type() {
        #[cfg(feature = "sql")]
        SQLITE_SOURCE_NAME => sqlite(source).await,

        #[cfg(feature = "sql")]
        MYSQL_SOURCE_NAME => mysql(source).await,

        #[cfg(feature = "sql")]
        POSTGRES_SOURCE_NAME => postgres(source).await,

        #[cfg(all(feature = "sql", feature = "mssql"))]
        MSSQL_SOURCE_NAME => mssql(source).await,

        x => Err(PrismaError::ConfigurationError(format!(
            "Unsupported connector type: {}",
            x
        ))),
    }
}

#[cfg(feature = "sql")]
async fn sqlite(
    source: &(dyn Source + Send + Sync),
) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    trace!("Loading SQLite connector...");

    let sqlite = Sqlite::from_source(source).await?;
    let path = PathBuf::from(sqlite.file_path());
    let db_name = path.file_stem().unwrap().to_str().unwrap().to_owned(); // Safe due to previous validations.

    trace!("Loaded SQLite connector.");
    Ok((db_name, sql_executor("sqlite", sqlite, false)))
}

#[cfg(feature = "sql")]
async fn postgres(
    source: &(dyn Source + Send + Sync),
) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    trace!("Loading Postgres connector...");

    let url = Url::parse(&source.url().value)?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let db_name = params
        .get("schema")
        .map(ToString::to_string)
        .unwrap_or_else(|| String::from("public"));

    let psql = PostgreSql::from_source(source).await?;

    let force_transactions = params
        .get("pgbouncer")
        .and_then(|flag| flag.parse().ok())
        .unwrap_or(false);

    trace!("Loaded Postgres connector.");
    Ok((db_name, sql_executor("postgres", psql, force_transactions)))
}

#[cfg(feature = "sql")]
async fn mysql(
    source: &(dyn Source + Send + Sync),
) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    trace!("Loading MySQL connector...");

    let mysql = Mysql::from_source(source).await?;
    let url = Url::parse(&source.url().value)?;
    let err_str = "No database found in connection string";

    let mut db_name = url
        .path_segments()
        .ok_or_else(|| PrismaError::ConfigurationError(err_str.into()))?;

    let db_name = db_name.next().expect(err_str).to_owned();

    trace!("Loaded MySQL connector.");
    Ok((db_name, sql_executor("mysql", mysql, false)))
}

#[cfg(all(feature = "sql", feature = "mssql"))]
async fn mssql(
    source: &(dyn Source + Send + Sync),
) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    trace!("Loading SQL Server connector...");

    let mssql = Mssql::from_source(source).await?;

    let mut splitted = source.url().value.split(";");
    splitted.next();

    let mut params: HashMap<String, String> = splitted
        .map(|kv| {
            let mut splitted = kv.split("=");
            let key = splitted.next().unwrap();
            let value = splitted.next().unwrap();

            (key.to_lowercase(), value.to_string())
        })
        .collect();

    let db_name = params.remove("database").unwrap_or_else(|| String::from("master"));

    trace!("Loaded SQL Server connector.");
    Ok((db_name, sql_executor("mssql", mssql, false)))
}

#[cfg(feature = "sql")]
fn sql_executor<T>(
    primary_connector: &'static str,
    connector: T,
    force_transactions: bool,
) -> Box<dyn QueryExecutor + Send + Sync + 'static>
where
    T: Connector + Send + Sync + 'static,
{
    Box::new(InterpretingExecutor::new(
        connector,
        primary_connector,
        force_transactions,
    ))
}
