use schema_core::json_rpc::types::*;
use std::path::Path;
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See ListMigrationDirectories::send()"]
pub struct ListMigrationDirectories<'a> {
    migrations_directory: &'a TempDir,
}

impl<'a> ListMigrationDirectories<'a> {
    pub fn new(migrations_directory: &'a TempDir) -> Self {
        ListMigrationDirectories { migrations_directory }
    }

    #[track_caller]
    pub fn send(self) -> ListMigrationDirectoriesAssertion<'a> {
        let migrations_from_filesystem = schema_core::schema_connector::migrations_directory::list_migrations(
            Path::new(self.migrations_directory.path()),
        )
        .unwrap();

        let migrations = migrations_from_filesystem
            .iter()
            .map(|migration| migration.migration_name().to_string())
            .collect();

        let output = ListMigrationDirectoriesOutput { migrations };

        ListMigrationDirectoriesAssertion {
            output,
            _migrations_directory: self.migrations_directory,
        }
    }
}

pub struct ListMigrationDirectoriesAssertion<'a> {
    output: ListMigrationDirectoriesOutput,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for ListMigrationDirectoriesAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListMigrationDirectoriesAssertion {{ .. }}")
    }
}

impl ListMigrationDirectoriesAssertion<'_> {
    #[track_caller]
    pub fn assert_listed_directories(self, names: &[&str]) -> Self {
        let found_names: Vec<&str> = self.output.migrations.iter().map(|name| &name[15..]).collect();

        assert!(
            found_names == names,
            "Assertion failed. The listed migrations do not match the expectations. ({found_names:?} vs {names:?})"
        );

        self
    }
}
