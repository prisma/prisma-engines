use super::MigrationCommand;
use crate::{api::MigrationApi, CoreResult};
use migration_connector::MigrationConnector;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The input to the `ListMigrationDirectories` command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListMigrationDirectoriesInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
}

/// The output of the `ListMigrationDirectories` command.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListMigrationDirectoriesOutput {
    /// The names of the migrations in the migration directory. Empty if no migrations are found.
    pub migrations: Vec<String>,
}

/// Reads the names of the subfolders in the migrations directory and returns their names.
pub struct ListMigrationDirectoriesCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for ListMigrationDirectoriesCommand {
    type Input = ListMigrationDirectoriesInput;
    type Output = ListMigrationDirectoriesOutput;

    async fn execute<C: MigrationConnector>(input: &Self::Input, engine: &MigrationApi<C>) -> CoreResult<Self::Output> {
        let migrations_from_filesystem =
            migration_connector::list_migrations(&Path::new(&input.migrations_directory_path))?;
        //Validate Provider
        migration_connector::error_on_changed_provider(
            &input.migrations_directory_path,
            engine.connector().connector_type(),
        )?;

        let migrations = migrations_from_filesystem
            .iter()
            .map(|migration| migration.migration_name().to_string())
            .collect();

        Ok(ListMigrationDirectoriesOutput { migrations })
    }
}
