use serde::Serialize;
use user_facing_error_macros::*;

/// [spec](https://github.com/prisma/specs/tree/master/errors#p3000-database-creation-failed)
#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P3000", message = "Failed to create database: {database_error}")]
pub struct DatabaseCreationFailed {
    pub database_error: String,
}

/// [spec](https://github.com/prisma/specs/tree/master/errors#p3001-destructive-migration-detected)
/// No longer used.
#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3001",
    message = "Migration possible with destructive changes and possible data loss: {destructive_details}"
)]
#[allow(dead_code)]
pub struct DestructiveMigrationDetected {
    pub destructive_details: String,
}

/// No longer used.
#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3002",
    message = "The attempted migration was rolled back: {database_error}"
)]
#[allow(dead_code)]
struct MigrationRollback {
    pub database_error: String,
}

/// No longer used.
#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3003",
    message = "The format of migrations changed, the saved migrations are no longer valid. To solve this problem, please follow the steps at: https://pris.ly/d/migrate"
)]
#[allow(dead_code)]
pub struct DatabaseMigrationFormatChanged;

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3004",
    message = "The `{database_name}` database is a system database, it should not be altered with prisma migrate. Please connect to another database."
)]
pub struct MigrateSystemDatabase {
    pub database_name: String,
}

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3005",
    message = "The database schema is not empty. Read more about how to baseline an existing production database: https://pris.ly/d/migrate-baseline"
)]
pub struct DatabaseSchemaNotEmpty;

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

        format!(
            "Migration `{migration_name}` failed to apply cleanly to the shadow database. \n{error_code}Error:\n{inner_error}",
            migration_name = self.migration_name,
            inner_error = self.inner_error.message(),
            error_code = error_code
        )
    }
}

#[derive(Debug, Serialize)]
pub struct PreviewFeaturesBlocked {
    pub features: Vec<String>,
}

impl crate::UserFacingError for PreviewFeaturesBlocked {
    const ERROR_CODE: &'static str = "P3007";

    fn message(&self) -> String {
        let blocked: Vec<_> = self.features.iter().map(|s| format!("`{s}`")).collect();

        format!(
            "Some of the requested preview features are not yet allowed in schema engine. Please remove them from your data model before using migrations. (blocked: {list_of_blocked_features})",
            list_of_blocked_features = blocked.join(", "),
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

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3010",
    message = "The name of the migration is too long. It must not be longer than 200 characters (bytes)."
)]
pub struct MigrationNameTooLong;

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3011",
    message = "Migration `{migration_name}` cannot be rolled back because it was never applied to the database. Hint: did you pass in the whole migration name? (example: \"20201207184859_initial_migration\")"
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

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3013",
    message = "Datasource provider arrays are no longer supported in migrate. Please change your datasource to use a single provider. Read more at https://pris.ly/multi-provider-deprecation"
)]
pub struct DeprecatedProviderArray;

#[derive(Debug, Serialize)]
pub struct ShadowDbCreationError {
    pub inner_error: crate::Error,
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
            "Prisma Migrate could not create the shadow database. Please make sure the database user has permission to create databases. Read more about the shadow database (and workarounds) at https://pris.ly/d/migrate-shadow\n\nOriginal error: {error_code}\n{inner_error}",
            inner_error = self.inner_error.message(),
            error_code = error_code
        )
    }
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3015",
    message = "Could not find the migration file at {migration_file_path}. Please delete the directory or restore the migration file."
)]
pub struct MigrationFileNotFound {
    pub migration_file_path: String,
}

#[derive(Debug, Serialize)]
pub struct SoftResetFailed {
    pub inner_error: crate::Error,
}

impl crate::UserFacingError for SoftResetFailed {
    const ERROR_CODE: &'static str = "P3016";

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
            "The fallback method for database resets failed, meaning Migrate could not clean up the database entirely. Original error: {error_code}\n{inner_error}",
            inner_error = self.inner_error.message(),
            error_code = error_code
        )
    }
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3017",
    message = "The migration {migration_name} could not be found. Please make sure that the migration exists, and that you included the whole name of the directory. (example: \"20201207184859_initial_migration\")"
)]
pub struct MigrationToMarkAppliedNotFound {
    /// The migration name that was provided, and not found.
    pub migration_name: String,
}

#[derive(Debug, Serialize, UserFacingError)]
#[user_facing(
    code = "P3018",
    message = "A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve\n\nMigration name: {migration_name}\n\nDatabase error code: {database_error_code}\n\nDatabase error:\n{database_error}
"
)]
pub struct ApplyMigrationError {
    pub migration_name: String,
    pub database_error_code: String,
    pub database_error: String,
}

#[derive(Debug, Serialize)]
pub struct ProviderSwitchedError {
    /// The provider specified in the schema.
    pub provider: String,
    /// The provider from migrate.lock
    pub expected_provider: String,
}

impl crate::UserFacingError for ProviderSwitchedError {
    const ERROR_CODE: &'static str = "P3019";

    fn message(&self) -> String {
        let provider = &self.provider;
        let expected_provider = &self.expected_provider;

        match (provider.as_str(), expected_provider.as_str()) {
            ("cockroachdb", "postgresql") => format!(
                "The datasource provider `{provider}` specified in your schema does not match the one specified in the migration_lock.toml, `{expected_provider}`. Check out the following documentation for how to resolve this: https://pris.ly/d/cockroachdb-postgresql-provider"
            ),
            _ => format!(
                "The datasource provider `{provider}` specified in your schema does not match the one specified in the migration_lock.toml, `{expected_provider}`. Please remove your current migration directory and start a new migration history with prisma migrate dev. Read more: https://pris.ly/d/migrate-provider-switch"
            ),
        }
    }
}

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3020",
    message = "The automatic creation of shadow databases is disabled on Azure SQL. Please set up a shadow database using the `shadowDatabaseUrl` datasource attribute.\nRead the docs page for more details: https://pris.ly/d/migrate-shadow"
)]
pub struct AzureMssqlShadowDb;

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3021",
    message = "Foreign keys cannot be created on this database. Learn more how to handle this: https://pris.ly/d/migrate-no-foreign-keys"
)]
pub struct ForeignKeyCreationNotAllowed;

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3022",
    message = "Direct execution of DDL (Data Definition Language) SQL statements is disabled on this database. Please read more here about how to handle this: https://pris.ly/d/migrate-no-direct-ddl"
)]
pub struct DirectDdlNotAllowed;

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3023",
    message = "For the current database dialect, `externalTables` & `externalEnums` in your prisma config must contain only fully qualified identifiers (e.g. `schema_name.table_name`)."
)]
pub struct MissingNamespaceInExternalTables;

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(
    code = "P3024",
    message = "For the current database dialect, `externalTables` & `externalEnums` in your prisma config must contain only simple identifiers without a schema name."
)]
pub struct UnexpectedNamespaceInExternalTables;

#[derive(Debug, SimpleUserFacingError)]
#[user_facing(code = "P4001", message = "The introspected database was empty.")]
pub struct IntrospectionResultEmpty;

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P4002",
    message = "The schema of the introspected database was inconsistent: {explanation}"
)]
pub struct DatabaseSchemaInconsistent {
    /// The schema was inconsistent and therefore introspection failed.
    pub explanation: String,
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
