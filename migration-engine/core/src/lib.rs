#![deny(rust_2018_idioms)]

pub mod api;
pub mod commands;
pub mod error;
pub mod migration;
pub mod migration_engine;

pub use api::GenericApi;
pub use commands::{ApplyMigrationInput, InferMigrationStepsInput, MigrationStepsResultOutput};
pub use error::CoreResult;

use commands::{CommandError, CommandResult};
use datamodel::{
    configuration::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    dml::Datamodel,
};
use error::Error;
use sql_migration_connector::SqlMigrationConnector;
use std::sync::Arc;

pub async fn migration_api(datamodel: &str) -> CoreResult<Arc<dyn api::GenericApi>> {
    let config = datamodel::parse_configuration(datamodel)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?;

    let connector = match source.connector_type() {
        #[cfg(feature = "sql")]
        provider if POSTGRES_SOURCE_NAME == provider => {
            let mut u = url::Url::parse(&source.url().value).unwrap();

            let params: Vec<(String, String)> = u.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

            u.query_pairs_mut().clear();

            for (k, v) in params.into_iter() {
                if k == "statement_cache_size" {
                    u.query_pairs_mut().append_pair("statement_cache_size", "0");
                } else {
                    u.query_pairs_mut().append_pair(&k, &v);
                }
            }

            if !u.query_pairs().any(|(k, _)| k == "statement_cache_size") {
                u.query_pairs_mut().append_pair("statement_cache_size", "0");
            }

            SqlMigrationConnector::new(u.as_str()).await?
        }
        #[cfg(feature = "sql")]
        provider if [MYSQL_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&provider) => {
            SqlMigrationConnector::new(&source.url().value).await?
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    };

    let api = api::MigrationApi::new(connector).await?;

    Ok(Arc::new(api))
}

pub async fn create_database(datamodel: &str) -> CoreResult<String> {
    let config = datamodel::parse_configuration(datamodel)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?;

    match source.connector_type() {
        provider if [MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&provider) => {
            Ok(SqlMigrationConnector::create_database(&source.url().value).await?)
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

pub(crate) fn parse_datamodel(datamodel: &str) -> CommandResult<Datamodel> {
    datamodel::parse_datamodel(&datamodel)
        .map_err(|err| CommandError::ReceivedBadDatamodel(err.to_pretty_string("schema.prisma", datamodel)))
}
