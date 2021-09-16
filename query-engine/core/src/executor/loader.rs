use super::{interpreting_executor::InterpretingExecutor, QueryExecutor};
use crate::CoreError;
use connection_string::JdbcString;
use connector::Connector;
use datamodel::{
    common::{
        preview_features::PreviewFeature,
        provider_names::{MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    },
    Datasource,
};
use sql_connector::*;
use std::collections::HashMap;
use std::str::FromStr;
use url::Url;

#[cfg(feature = "mongodb")]
use datamodel::common::provider_names::MONGODB_SOURCE_NAME;
#[cfg(feature = "mongodb")]
use mongodb_connector::MongoDb;

const DEFAULT_SQLITE_DB_NAME: &str = "main";

/// Loads a query executor based on the parsed Prisma schema (datasource).
#[tracing::instrument(name = "exec_loader", skip(source))]
pub async fn load(
    source: &Datasource,
    features: &[PreviewFeature],
    url: &str,
) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    match source.active_provider.as_str() {
        SQLITE_SOURCE_NAME => sqlite(source, url).await,
        MYSQL_SOURCE_NAME => mysql(source, url).await,
        POSTGRES_SOURCE_NAME => postgres(source, url).await,
        MSSQL_SOURCE_NAME => mssql(source, url).await,

        #[cfg(feature = "mongodb")]
        MONGODB_SOURCE_NAME => {
            if !features.contains(&PreviewFeature::MongoDb) {
                let error = CoreError::UnsupportedFeatureError(
                    "MongoDB query connector (experimental feature, needs to be enabled)".into(),
                );

                return Err(error);
            }

            mongodb(source, url).await
        }

        x => Err(CoreError::ConfigurationError(format!(
            "Unsupported connector type: {}",
            x
        ))),
    }
}

pub fn db_name(source: &Datasource, url: &str) -> crate::Result<String> {
    match source.active_provider.as_str() {
        SQLITE_SOURCE_NAME => Ok(DEFAULT_SQLITE_DB_NAME.to_string()),
        MYSQL_SOURCE_NAME => {
            let url = Url::parse(url)?;
            let err_str = "No database found in connection string";

            let mut db_name = url
                .path_segments()
                .ok_or_else(|| CoreError::ConfigurationError(err_str.into()))?;

            let db_name = db_name.next().expect(err_str).to_owned();

            Ok(db_name)
        }
        POSTGRES_SOURCE_NAME => {
            let url = Url::parse(url)?;
            let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

            let db_name = params
                .get("schema")
                .map(ToString::to_string)
                .unwrap_or_else(|| String::from("public"));

            Ok(db_name)
        }
        MSSQL_SOURCE_NAME => {
            let mut conn = JdbcString::from_str(&format!("jdbc:{}", url))?;
            let db_name = conn
                .properties_mut()
                .remove("schema")
                .unwrap_or_else(|| String::from("dbo"));

            Ok(db_name)
        }
        #[cfg(feature = "mongodb")]
        MONGODB_SOURCE_NAME => {
            let url = Url::parse(url)?;
            let database = url.path().trim_start_matches('/').to_string();

            Ok(database)
        }
        x => Err(CoreError::ConfigurationError(format!(
            "Unsupported connector type: {}",
            x
        ))),
    }
}

async fn sqlite(source: &Datasource, url: &str) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading SQLite query connector...");

    let sqlite = Sqlite::from_source(source, url).await?;
    let db_name = db_name(source, url)?;

    trace!("Loaded SQLite query connector.");
    Ok((db_name, sql_executor(sqlite, false)))
}

async fn postgres(source: &Datasource, url: &str) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading Postgres query connector...");

    let database_str = url;
    let psql = PostgreSql::from_source(source, url).await?;

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

async fn mysql(source: &Datasource, url: &str) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading MySQL query connector...");

    let mysql = Mysql::from_source(source, url).await?;
    let db_name = db_name(source, url)?;

    trace!("Loaded MySQL query connector.");
    Ok((db_name, sql_executor(mysql, false)))
}

async fn mssql(source: &Datasource, url: &str) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading SQL Server query connector...");

    let mssql = Mssql::from_source(source, url).await?;
    let db_name = db_name(source, url)?;

    trace!("Loaded SQL Server query connector.");
    Ok((db_name, sql_executor(mssql, false)))
}

fn sql_executor<T>(connector: T, force_transactions: bool) -> Box<dyn QueryExecutor + Send + Sync>
where
    T: Connector + Send + Sync + 'static,
{
    Box::new(InterpretingExecutor::new(connector, force_transactions))
}

#[cfg(feature = "mongodb")]
async fn mongodb(source: &Datasource, url: &str) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading MongoDB query connector...");

    let mongo = MongoDb::new(source, url).await?;
    let db_name = db_name(source, url)?;

    trace!("Loaded MongoDB query connector.");
    Ok((db_name.to_owned(), Box::new(InterpretingExecutor::new(mongo, false))))
}
