use serde::{Deserialize, Serialize};

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
