use serde::Serialize;
use user_facing_error_macros::*;

/// [spec](https://github.com/prisma/specs/tree/master/errors#p3000-database-creation-failed)
#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P3000", message = "Failed to create database: ${database_error}")]
pub struct DatabaseCreationFailed {
    pub database_error: String,
}

/// [spec](https://github.com/prisma/specs/tree/master/errors#p3001-destructive-migration-detected)
#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3001",
    message = "Migration possible with destructive changes and possible data loss: ${migration_engine_destructive_details}"
)]
pub struct DestructiveMigrationDetected {
    pub migration_engine_destructive_details: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P3002",
    message = "The attempted migration was rolled back: ${database_error}"
)]
struct MigrationRollback {
    pub database_error: String,
}

// Tests

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
