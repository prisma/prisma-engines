use crate::{migrations_directory::MigrationDirectory, ConnectorResult};
use datamodel::{Configuration, Datamodel};

/// The component responsible for generating a
/// [DatabaseMigration](trait.MigrationConnector.html#associatedtype.DatabaseMigration)
/// migrating the database from one datamodel to another.
///
/// In addition to the datamodel information provided by the core, a connector
/// may gather additional information itself, e.g. through looking at the
/// description of the underlying database.
#[async_trait::async_trait]
pub trait DatabaseMigrationInferrer<T>: Send + Sync {
    /// Infer the database migration to the passed in Prisma schema.
    async fn infer(&self, next: (&Configuration, &Datamodel)) -> ConnectorResult<T>;

    /// Infer the database migration steps assuming an empty schema on a new
    /// database as a starting point.
    fn infer_from_empty(&self, next: (&Configuration, &Datamodel)) -> ConnectorResult<T>;

    /// Look at the previous migrations and the target Prisma schema, and infer
    /// a database migration taking the database to the target Prisma schema.
    async fn infer_next_migration(
        &self,
        previous_migrations: &[MigrationDirectory],
        target_schema: (&Configuration, &Datamodel),
    ) -> ConnectorResult<T>;

    /// Check that the current local database's schema matches its expected
    /// state at the end of the passed in migrations history. If there is drift,
    /// it should return a script to attempt to correct it.
    async fn calculate_drift(&self, applied_migrations: &[MigrationDirectory]) -> ConnectorResult<Option<String>>;

    /// If possible, check that the passed in migrations apply cleanly.
    async fn validate_migrations(&self, migrations: &[MigrationDirectory]) -> ConnectorResult<()>;
}
