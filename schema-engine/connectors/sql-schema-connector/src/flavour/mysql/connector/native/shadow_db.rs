use crate::flavour::{MysqlConnector, SqlConnector, mysql};
use schema_connector::{ConnectorResult, migrations_directory::MigrationDirectory};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migrations_history(
    migrations: &[MigrationDirectory],
    shadow_db: &mut MysqlConnector,
) -> ConnectorResult<SqlSchema> {
    for migration in migrations {
        let script = migration.read_migration_script()?;

        tracing::debug!(
            "Applying migration `{}` to shadow database.",
            migration.migration_name()
        );

        mysql::scan_migration_script_impl(&script);

        shadow_db
            .apply_migration_script(migration.migration_name(), &script)
            .await
            .map_err(|connector_error| {
                connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
            })?;
    }

    shadow_db.describe_schema(None).await
}
