use crate::flavour::postgres::PostgresProvider::CockroachDb;
use crate::flavour::{PostgresFlavour, SqlFlavour};
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use schema_connector::{ConnectorError, Namespaces};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migrations_history(
    migrations: &[MigrationDirectory],
    mut shadow_db: PostgresFlavour,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    if shadow_db.provider == CockroachDb {
        // CockroachDB is very slow in applying DDL statements.
        // A workaround to it is to run the statements in a transaction block. This comes with some
        // drawbacks and limitations though, so we only apply this when creating a shadow db.
        // See https://www.cockroachlabs.com/docs/stable/online-schema-changes#limitations
        // Original GitHub issue with context: https://github.com/prisma/prisma/issues/12384#issuecomment-1152523689
        shadow_db.raw_cmd("BEGIN;").await?;
    }

    for migration in migrations {
        let script = migration.read_migration_script()?;

        tracing::debug!(
            "Applying migration `{}` to shadow database.",
            migration.migration_name()
        );

        shadow_db
            .raw_cmd(&script)
            .await
            .map_err(ConnectorError::from)
            .map_err(|connector_error| {
                connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
            })?;
    }

    if shadow_db.provider == CockroachDb {
        shadow_db.raw_cmd("COMMIT;").await?;
    }

    shadow_db.describe_schema(namespaces).await
}
