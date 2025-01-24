use crate::flavour::postgres::{Circumstances, MigratePostgresUrl};
use crate::flavour::{validate_connection_infos_do_not_match, PostgresFlavour, SqlFlavour};
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use schema_connector::{ConnectorError, ConnectorParams, Namespaces};
use sql_schema_describer::SqlSchema;
use url::Url;

use super::{Connection, PostgresProvider};

pub async fn sql_schema_from_migration_history(
    state: &mut super::State,
    provider: PostgresProvider,
    migrations: &[MigrationDirectory],
    shadow_database_connection_string: Option<String>,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    let shadow_database_connection_string = shadow_database_connection_string.or_else(|| {
        state
            .params()
            .and_then(|p| p.connector_params.shadow_database_connection_string.clone())
    });
    let mut shadow_database = PostgresFlavour::default();

    let is_postgres = provider == PostgresProvider::PostgreSql
        && super::get_circumstances(state).is_none_or(|c| !c.contains(Circumstances::IsCockroachDb));

    match shadow_database_connection_string {
        Some(shadow_database_connection_string) => {
            if let Some(params) = state.params() {
                validate_connection_infos_do_not_match(
                    &shadow_database_connection_string,
                    &params.connector_params.connection_string,
                )?;
            }

            let shadow_db_params = ConnectorParams {
                connection_string: shadow_database_connection_string,
                preview_features: state
                    .params()
                    .map(|p| p.connector_params.preview_features)
                    .unwrap_or_default(),
                shadow_database_connection_string: None,
            };

            shadow_database.set_params(shadow_db_params)?;
            shadow_database.ensure_connection_validity().await?;

            tracing::info!("Connecting to user-provided shadow database.");

            if shadow_database.reset(namespaces.clone()).await.is_err() {
                crate::best_effort_reset(&mut shadow_database, namespaces.clone()).await?;
            }

            sql_schema_from_migrations_and_db(migrations, shadow_database, namespaces).await
        }
        None => {
            let (main_connection, params) = super::get_connection_and_params(state, provider).await?;
            let shadow_database_name = crate::new_shadow_database_name();

            {
                let create_database = format!("CREATE DATABASE \"{shadow_database_name}\"");
                main_connection
                    .raw_cmd(&create_database)
                    .await
                    .map_err(|err| super::quaint_error_mapper(params)(err).into_shadow_db_creation_error())?;
            }

            let mut shadow_database_url: Url = params
                .connector_params
                .connection_string
                .parse()
                .map_err(ConnectorError::url_parse_error)?;

            if shadow_database_url.scheme() == MigratePostgresUrl::WEBSOCKET_SCHEME {
                shadow_database_url
                    .query_pairs_mut()
                    .append_pair(MigratePostgresUrl::DBNAME_PARAM, &shadow_database_name);
            } else {
                shadow_database_url.set_path(&format!("/{shadow_database_name}"));
            }
            let shadow_db_params = ConnectorParams {
                connection_string: shadow_database_url.to_string(),
                preview_features: params.connector_params.preview_features,
                shadow_database_connection_string: None,
            };
            shadow_database.set_params(shadow_db_params)?;
            tracing::debug!("Connecting to shadow database `{}`", shadow_database_name);
            shadow_database.ensure_connection_validity().await?;

            // We go through the whole process without early return, then clean up
            // the shadow database, and only then return the result. This avoids
            // leaving shadow databases behind in case of e.g. faulty migrations.
            let ret = sql_schema_from_migrations_and_db(migrations, shadow_database, namespaces).await;

            if is_postgres {
                drop_db_try_force(main_connection, &shadow_database_name)
                    .await
                    .map_err(super::quaint_error_mapper(params))?;
            } else {
                let drop_database = format!("DROP DATABASE IF EXISTS \"{shadow_database_name}\"");
                main_connection
                    .raw_cmd(&drop_database)
                    .await
                    .map_err(super::quaint_error_mapper(params))?;
            }

            ret
        }
    }
}

pub async fn sql_schema_from_migrations_and_db(
    migrations: &[MigrationDirectory],
    mut shadow_db: PostgresFlavour,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
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

    shadow_db.describe_schema(namespaces).await
}

/// Drop a database using `WITH (FORCE)` syntax.
///
/// When drop database is routed through pgbouncer, the database may still be used in other pooled connections.
/// In this case, given that we (as a user) know the database will not be used any more, we can forcefully drop
/// the database. Note that `with (force)` is added in Postgres 13, and therefore we will need to
/// fallback to the normal drop if it errors with syntax error.
///
/// TL;DR,
/// 1. pg >= 13 -> it works.
/// 2. pg < 13 -> syntax error on WITH (FORCE), and then fail with db in use if pgbouncer is used.
async fn drop_db_try_force(conn: &Connection, database_name: &str) -> quaint::Result<()> {
    let drop_database = format!("DROP DATABASE IF EXISTS \"{database_name}\" WITH (FORCE)");
    if let Err(err) = conn.raw_cmd(&drop_database).await {
        if let Some(msg) = err.original_message() {
            if msg.contains("syntax error") {
                let drop_database_alt = format!("DROP DATABASE IF EXISTS \"{database_name}\"");
                conn.raw_cmd(&drop_database_alt).await?;
            } else {
                return Err(err);
            }
        } else {
            return Err(err);
        }
    }
    Ok(())
}
