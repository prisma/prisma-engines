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

pub fn load(source: &dyn Source) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    match source.connector_type() {
        #[cfg(feature = "sql")]
        SQLITE_SOURCE_NAME => sqlite(source),

        #[cfg(feature = "sql")]
        MYSQL_SOURCE_NAME => mysql(source),

        #[cfg(feature = "sql")]
        POSTGRES_SOURCE_NAME => postgres(source),

        x => Err(PrismaError::ConfigurationError(format!(
            "Unsupported connector type: {}",
            x
        ))),
    }
}

#[cfg(feature = "sql")]
fn sqlite(source: &dyn Source) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    trace!("Loading SQLite connector...");

    let sqlite = Sqlite::from_source(source)?;
    let path = PathBuf::from(sqlite.file_path());
    let db_name = path.file_stem().unwrap().to_str().unwrap().to_owned(); // Safe due to previous validations.

    trace!("Loaded SQLite connector.");
    Ok((db_name, sql_executor("sqlite", sqlite)))
}

#[cfg(feature = "sql")]
fn postgres(source: &dyn Source) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    trace!("Loading Postgres connector...");

    let url = Url::parse(&source.url().value)?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let db_name = params
        .get("schema")
        .map(ToString::to_string)
        .unwrap_or_else(|| String::from("public"));

    let psql = PostgreSql::from_source(source)?;

    trace!("Loaded Postgres connector.");
    Ok((db_name, sql_executor("postgres", psql)))
}

#[cfg(feature = "sql")]
fn mysql(source: &dyn Source) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync + 'static>)> {
    trace!("Loading MySQL connector...");

    let mysql = Mysql::from_source(source)?;
    let url = Url::parse(&source.url().value)?;
    let err_str = "No database found in connection string";

    let mut db_name = url
        .path_segments()
        .ok_or_else(|| PrismaError::ConfigurationError(err_str.into()))?;

    let db_name = db_name.next().expect(err_str).to_owned();

    trace!("Loaded MySQL connector.");
    Ok((db_name, sql_executor("mysql", mysql)))
}

#[cfg(feature = "sql")]
fn sql_executor<T>(primary_connector: &'static str, connector: T) -> Box<dyn QueryExecutor + Send + Sync + 'static>
where
    T: Connector + Send + Sync + 'static,
{
    Box::new(InterpretingExecutor::new(connector, primary_connector))
}
