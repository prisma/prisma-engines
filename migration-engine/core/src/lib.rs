#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod api;
pub mod commands;
pub mod error;
pub mod migration;
pub mod migration_engine;

mod migrations_directory;

pub use api::GenericApi;
pub use commands::{ApplyMigrationInput, InferMigrationStepsInput, MigrationStepsResultOutput};
pub use error::CoreResult;

use commands::{CommandError, CommandResult};
use datamodel::{
    common::provider_names::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    dml::Datamodel,
};
use error::Error;
use migration_connector::ConnectorError;
use sql_migration_connector::SqlMigrationConnector;
use std::sync::Arc;

/// Top-level constructor for the migration engine API.
pub async fn migration_api(datamodel: &str) -> CoreResult<Arc<dyn api::GenericApi>> {
    let config = datamodel::parse_configuration(datamodel)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?;

    let connector = match &source.active_provider {
        #[cfg(feature = "sql")]
        provider if POSTGRES_SOURCE_NAME == provider => {
            let mut u = url::Url::parse(&source.url().value).map_err(|url_error| {
                Error::ConnectorError(ConnectorError::url_parse_error(url_error, &source.url().value))
            })?;

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
        provider if [MYSQL_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&provider.as_str()) => {
            SqlMigrationConnector::new(&source.url().value).await?
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    };

    let api = api::MigrationApi::new(connector).await?;

    Ok(Arc::new(api))
}

/// Create the database referenced by the passed in Prisma schema.
pub async fn create_database(schema: &str) -> CoreResult<String> {
    let config = datamodel::parse_configuration(schema)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?;

    match &source.active_provider {
        provider if [MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&provider.as_str()) => {
            Ok(SqlMigrationConnector::create_database(&source.url().value).await?)
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

/// Database setup for connector-test-kit.
pub async fn qe_setup(prisma_schema: &str) -> CoreResult<()> {
    let config = datamodel::parse_configuration(prisma_schema)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?;

    match &source.active_provider {
        provider if [MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&provider.as_str()) => {
            SqlMigrationConnector::qe_setup(&source.url().value).await?;
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    }

    Ok(())
}

pub(crate) fn parse_datamodel(datamodel: &str) -> CommandResult<Datamodel> {
    datamodel::parse_datamodel(&datamodel)
        .map_err(|err| CommandError::ReceivedBadDatamodel(err.to_pretty_string("schema.prisma", datamodel)))
}
