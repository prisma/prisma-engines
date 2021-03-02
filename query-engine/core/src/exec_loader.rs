use crate::{
    executor::{InterpretingExecutor, QueryExecutor},
    CoreError,
};
use connection_string::JdbcString;
use connector::Connector;
use std::str::FromStr;

use datamodel::{
    common::provider_names::{MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    Datasource,
};
use std::collections::HashMap;
use url::Url;

use sql_connector::*;

#[cfg(feature = "mongodb")]
use datamodel::common::provider_names::MONGODB_SOURCE_NAME;
#[cfg(feature = "mongodb")]
use mongodb_connector::MongoDb;

const DEFAULT_SQLITE_DB_NAME: &str = "main";

#[tracing::instrument(name = "exec_loader", skip(source))]
pub async fn load(source: &Datasource) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    match source.active_provider.as_str() {
        SQLITE_SOURCE_NAME => sqlite(source).await,
        MYSQL_SOURCE_NAME => mysql(source).await,
        POSTGRES_SOURCE_NAME => postgres(source).await,

        MSSQL_SOURCE_NAME => {
            if !feature_flags::get().microsoftSqlServer {
                let error = CoreError::UnsupportedFeatureError(
                    "Microsoft SQL Server (experimental feature, needs to be enabled)".into(),
                );

                return Err(error);
            }

            mssql(source).await
        }

        #[cfg(feature = "mongodb")]
        MONGODB_SOURCE_NAME => {
            if !feature_flags::get().mongoDb {
                let error = CoreError::UnsupportedFeatureError(
                    "MongoDB query connector (experimental feature, needs to be enabled)".into(),
                );

                return Err(error);
            }

            mongodb(source).await
        }

        x => Err(CoreError::ConfigurationError(format!(
            "Unsupported connector type: {}",
            x
        ))),
    }
}

async fn sqlite(source: &Datasource) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading SQLite query connector...");

    let sqlite = Sqlite::from_source(source).await?;
    let db_name = DEFAULT_SQLITE_DB_NAME.to_owned();

    trace!("Loaded SQLite query connector.");
    Ok((db_name, sql_executor(sqlite, false)))
}

async fn postgres(source: &Datasource) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading Postgres query connector...");

    let database_str = &source.url().value;
    let psql = PostgreSql::from_source(source).await?;

    let url = Url::parse(database_str)?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let db_name = params
        .get("schema")
        .map(ToString::to_string)
        .unwrap_or_else(|| String::from("public"));

    let force_transactions = params
        .get("pgbouncer")
        .and_then(|flag| flag.parse().ok())
        .unwrap_or(false);

    trace!("Loaded Postgres query connector.");
    Ok((db_name, sql_executor(psql, force_transactions)))
}

async fn mysql(source: &Datasource) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading MySQL query connector...");

    let mysql = Mysql::from_source(source).await?;
    let database_str = &source.url().value;

    let url = Url::parse(database_str)?;
    let err_str = "No database found in connection string";

    let mut db_name = url
        .path_segments()
        .ok_or_else(|| CoreError::ConfigurationError(err_str.into()))?;

    let db_name = db_name.next().expect(err_str).to_owned();

    trace!("Loaded MySQL query connector.");
    Ok((db_name, sql_executor(mysql, false)))
}

async fn mssql(source: &Datasource) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading SQL Server query connector...");

    let mssql = Mssql::from_source(source).await?;

    let mut conn = JdbcString::from_str(&format!("jdbc:{}", &source.url().value))?;
    let db_name = conn
        .properties_mut()
        .remove("schema")
        .unwrap_or_else(|| String::from("dbo"));

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
async fn mongodb(source: &Datasource) -> crate::Result<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading MongoDB query connector...");

    let mongo = MongoDb::new(source).await?;
    let db_name = mongo.db_name();

    trace!("Loaded MongoDB query connector.");
    Ok((db_name.to_owned(), Box::new(InterpretingExecutor::new(mongo, false))))
}
