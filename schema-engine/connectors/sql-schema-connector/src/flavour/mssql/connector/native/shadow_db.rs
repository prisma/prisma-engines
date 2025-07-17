use crate::flavour::{MssqlConnector, SqlConnector};
use schema_connector::{ConnectorResult, Namespaces, migrations_directory::MigrationDirectory};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migrations_history(
    migrations: &[MigrationDirectory],
    shadow_db: &mut MssqlConnector,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    for migration in migrations {
        let script = migration.read_migration_script()?;

        tracing::debug!(
            "Applying migration `{}` to shadow database.",
            migration.migration_name()
        );

        shadow_db.raw_cmd(&script).await.map_err(|connector_error| {
            connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
        })?;
    }

    shadow_db.describe_schema(namespaces).await
}
