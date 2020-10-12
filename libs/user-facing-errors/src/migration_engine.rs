use serde::Serialize;
use user_facing_error_macros::*;

/// [spec](https://github.com/prisma/specs/tree/master/errors#p3000-database-creation-failed)
#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P3000", message = "Failed to create database: {database_error}")]
pub struct DatabaseCreationFailed {
    pub database_error: String,
}

/// [spec](https://github.com/prisma/specs/tree/master/errors#p3001-destructive-migration-detected)
#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3001",
    message = "Migration possible with destructive changes and possible data loss: {migration_engine_destructive_details}"
)]
pub struct DestructiveMigrationDetected {
    pub migration_engine_destructive_details: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3002",
    message = "The attempted migration was rolled back: {database_error}"
)]
struct MigrationRollback {
    pub database_error: String,
}

// No longer used.
#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3003",
    message = "The format of migrations changed, the saved migrations are no longer valid. To solve this problem, please follow the steps at: https://pris.ly/d/migrate#troubleshooting"
)]
pub struct DatabaseMigrationFormatChanged;

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3004",
    message = "The `{database_name}` database is a system database, it should not be altered with prisma migrate. Please connect to another database."
)]
pub struct MigrateSystemDatabase {
    pub database_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3005",
    message = "The database schema for `{database_name}` is not empty. Please follow the to-be-written instructions on how to set up migrate with an existing database, or use an empty database."
)]
pub struct DatabaseSchemaNotEmpty {
    pub database_name: String,
}

#[derive(Debug, Serialize)]
pub struct MigrationDoesNotApplyCleanly {
    pub migration_name: String,
    pub inner_error: crate::Error,
}

impl crate::UserFacingError for MigrationDoesNotApplyCleanly {
    const ERROR_CODE: &'static str = "P3006";

    fn message(&self) -> String {
        format!("Migration `{migration_name}` failed to apply cleanly to a temporary database. \n{error_code}Error:\n{inner_error}", migration_name = self.migration_name, inner_error = self.inner_error.message(), error_code = match &self.inner_error.inner {
            crate::ErrorType::Known(crate::KnownError {
                message: _,
                meta: _,
                error_code,
            }) => format!("Error code: {}\n", &error_code),
            crate::ErrorType::Unknown(_) => String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UserFacingError;

    #[test]
    fn test_user_facing_error_impl_for_database_creation_failed() {
        assert_eq!(DatabaseCreationFailed::ERROR_CODE, "P3000");

        let error = DatabaseCreationFailed {
            database_error: "oops".to_string(),
        };

        assert_eq!(error.message(), "Failed to create database: oops")
    }
}
