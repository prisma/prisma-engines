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
    message = "The database schema for `{database_name}` is not empty. Read more about how to baseline an existing production database: https://pris.ly/d/migrate-baseline"
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
        let error_code = match &self.inner_error.inner {
            crate::ErrorType::Known(crate::KnownError {
                message: _,
                meta: _,
                error_code,
            }) => format!("Error code: {}\n", &error_code),
            crate::ErrorType::Unknown(_) => String::new(),
        };

        format!("Migration `{migration_name}` failed to apply cleanly to a temporary database. \n{error_code}Error:\n{inner_error}", migration_name = self.migration_name, inner_error = self.inner_error.message(), error_code = error_code)
    }
}

#[derive(Debug, Serialize)]
pub struct PreviewFeaturesBlocked {
    pub features: Vec<String>,
}

impl crate::UserFacingError for PreviewFeaturesBlocked {
    const ERROR_CODE: &'static str = "P3007";

    fn message(&self) -> String {
        let blocked: Vec<_> = self.features.iter().map(|s| format!("`{}`", s)).collect();

        format!(
            "Some of the requested preview features are not yet allowed in migration engine. Please remove them from your data model before using migrations. (blocked: {})",
            blocked.join(", "),
        )
    }
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3008",
    message = "The migration `{migration_name}` is already recorded as applied in the database."
)]
pub struct MigrationAlreadyApplied {
    /// The name of the migration.
    pub migration_name: String,
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3009",
    message = "migrate found failed migrations in the target database, new migrations will not be applied. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve\n{details}"
)]
pub struct FoundFailedMigrations {
    /// The details about each failed migration.
    pub details: String,
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3010",
    message = "The name of the migration is too long. It must not be longer than 200 characters (bytes)."
)]
pub struct MigrationNameTooLong;

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3011",
    message = "Migration `{migration_name}` cannot be rolled back because it was never applied to the database."
)]
pub struct CannotRollBackUnappliedMigration {
    /// The name of the migration.
    pub migration_name: String,
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3012",
    message = "Migration `{migration_name}` cannot be rolled back because it is not in a failed state."
)]
pub struct CannotRollBackSucceededMigration {
    /// The name of the migration.
    pub migration_name: String,
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3013",
    message = "Datasource provider arrays are no longer supported in migrate. Please change your datasource to use a single provider. Read more at https://pris.ly/multi-provider-deprecation"
)]
pub struct DeprecatedProviderArray;

#[derive(Debug, Serialize)]
pub struct ShadowDbCreationError {
    pub inner_error: crate::Error,
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3014",
    message = "The datasource provider `{provider}` specified in your schema does not match the one specified in the migration_lock.toml. You will encounter errors when you try to apply migrations generated for a different provider. Please archive your current migration directory at a different location and start a new migration history with `prisma migrate dev`."
)]
pub struct ProviderSwitchedError {
    ///The provider specified in the schema.
    pub provider: String,
}
impl crate::UserFacingError for ShadowDbCreationError {
    const ERROR_CODE: &'static str = "P3014";

    fn message(&self) -> String {
        let error_code = match &self.inner_error.inner {
            crate::ErrorType::Known(crate::KnownError {
                message: _,
                meta: _,
                error_code,
            }) => format!("Error code: {}\n", &error_code),
            crate::ErrorType::Unknown(_) => String::new(),
        };

        format!(
            "Prisma Migrate could not create the shadow database. Please make sure the database user has permission to create databases.  More info: https://pris.ly/d/migrate-shadow. Original error: {error_code}\n{inner_error}",
            inner_error = self.inner_error.message(),
            error_code = error_code
        )
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
