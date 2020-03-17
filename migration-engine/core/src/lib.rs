//! This library API is meant for the `test-cli` binary and migration-engine-tests only.

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
use std::sync::Arc;

pub async fn migration_api(datamodel: &str) -> CoreResult<Arc<dyn api::GenericApi>> {
    let config = datamodel::parse_configuration(datamodel)?;

    let source = config
        .datasources
        .first()
        .ok_or_else(|| CommandError::Generic(anyhow::anyhow!("There is no datasource in the schema.")))?;

    let connector = match source.connector_type() {
        #[cfg(feature = "sql")]
        provider if [MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME].contains(&provider) => {
            sql_migration_connector::SqlMigrationConnector::new(&source.url().value, provider).await?
        }
        x => unimplemented!("Connector {} is not supported yet", x),
    };

    let api = api::MigrationApi::new(connector).await?;

    Ok(Arc::new(api))
}

pub(crate) fn parse_datamodel(datamodel: &str) -> CommandResult<Datamodel> {
    datamodel::parse_datamodel(&datamodel)
        .map_err(|err| CommandError::ReceivedBadDatamodel(err.to_pretty_string("schema.prisma", datamodel)))
}
