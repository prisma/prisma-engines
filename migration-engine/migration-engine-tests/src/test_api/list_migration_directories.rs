use migration_core::{CoreResult, GenericApi};
use tempfile::TempDir;

use crate::AssertionResult;
use migration_core::commands::{ListMigrationDirectoriesInput, ListMigrationDirectoriesOutput};

#[must_use = "This struct does nothing on its own. See ListMigrationDirectories::send()"]
pub struct ListMigrationDirectories<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
}

impl<'a> ListMigrationDirectories<'a> {
    pub fn new(api: &'a dyn GenericApi, migrations_directory: &'a TempDir) -> Self {
        ListMigrationDirectories {
            api,
            migrations_directory,
        }
    }

    pub async fn send(self) -> CoreResult<ListMigrationDirectoriesAssertion<'a>> {
        let output = self
            .api
            .list_migration_directories(&ListMigrationDirectoriesInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
            })
            .await?;

        Ok(ListMigrationDirectoriesAssertion {
            output,
            _api: self.api,
            _migrations_directory: self.migrations_directory,
        })
    }
}

pub struct ListMigrationDirectoriesAssertion<'a> {
    output: ListMigrationDirectoriesOutput,
    _api: &'a dyn GenericApi,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for ListMigrationDirectoriesAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListMigrationDirectoriesAssertion {{ .. }}")
    }
}

impl<'a> ListMigrationDirectoriesAssertion<'a> {
    pub fn assert_listed_directories(self, names: &[&str]) -> AssertionResult<Self> {
        let found_names: Vec<&str> = self.output.migrations.iter().map(|name| &name[15..]).collect();

        anyhow::ensure!(
            found_names == names,
            "Assertion failed. The listed migrations do not match the expectations. ({:?} vs {:?})",
            found_names,
            names
        );

        Ok(self)
    }
}
