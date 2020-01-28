//! This library API is meant for the `test-cli` binary and migration-engine-tests only.

pub mod api;
pub mod commands;
pub mod error;
pub mod migration;
pub mod migration_engine;

pub use api::GenericApi;
pub use commands::{ApplyMigrationInput, InferMigrationStepsInput, MigrationStepsResultOutput};

use commands::{CommandError, CommandResult};
use datamodel::{
    configuration::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    dml::Datamodel,
};
use error::Error;

pub async fn migration_api(datamodel: &str) -> CoreResult<Box<dyn api::GenericApi>> {
    let config = datamodel::parse_configuration(datamodel)?;

    let source = config.datasources.first().ok_or(CommandError::DataModelErrors {
        errors: vec!["There is no datasource in the configuration.".to_string()],
    })?;

    let connector = match source.connector_type() {
        #[cfg(feature = "sql")]
        provider if [MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&provider) => {
            sql_migration_connector::SqlMigrationConnector::new(&source.url().value, provider).await?
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    };

    let api = api::MigrationApi::new(connector).await?;

    Ok(Box::new(api))
}

pub type CoreResult<T> = Result<T, Error>;

pub(crate) fn parse_datamodel(datamodel: &str) -> CommandResult<Datamodel> {
    let result = datamodel::parse_datamodel_or_pretty_error(&datamodel, "datamodel file, line");
    result.map_err(|e| CommandError::DataModelErrors { errors: vec![e] })
}
