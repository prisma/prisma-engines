use super::MigrationCommand;
use crate::{api::MigrationApi, core_error::CoreResult, CoreError};
use migration_connector::MigrationConnector;
use serde::Deserialize;
use std::path::Path;
use user_facing_errors::migration_engine::{
    ProviderSwitchedWithExistingLockFile, ProviderSwitchedWithExistingMigrations,
};

/// Method called if the specified database provider is to be changed. It will update the schema_lock.toml accordingly.
pub struct SwitchProviderCommand;

/// The `switchProvider` input.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchProviderInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for SwitchProviderCommand {
    type Input = SwitchProviderInput;
    type Output = ();

    async fn execute<C: MigrationConnector>(input: &Self::Input, engine: &MigrationApi<C>) -> CoreResult<Self::Output> {
        let migrations_from_filesystem =
            migration_connector::list_migrations(&Path::new(&input.migrations_directory_path))?;

        if !migrations_from_filesystem.is_empty() {
            return Err(CoreError::user_facing(ProviderSwitchedWithExistingMigrations));
        }

        if migration_connector::match_provider_in_lock_file(
            &input.migrations_directory_path,
            engine.connector().connector_type(),
        )
        .is_some()
        {
            return Err(CoreError::user_facing(ProviderSwitchedWithExistingLockFile));
        }

        migration_connector::write_migration_lock_file(
            &input.migrations_directory_path,
            engine.connector().connector_type(),
        )
        .unwrap();

        Ok(())
    }
}
