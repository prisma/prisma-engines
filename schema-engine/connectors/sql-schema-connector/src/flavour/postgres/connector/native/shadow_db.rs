use crate::flavour::postgres::{sql_schema_from_migrations_and_db, MigratePostgresUrl, PpgParams};
use crate::flavour::{PostgresConnector, SqlConnector, UsingExternalShadowDb};
use psl::PreviewFeatures;
use quaint::connector::is_url_localhost;
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use schema_connector::{ConnectorError, ConnectorParams, Namespaces};
use sql_schema_describer::SqlSchema;
use url::Url;

use super::{Connection, PostgresProvider};

pub async fn sql_schema_from_migration_history(
    connector: &mut PostgresConnector,
    provider: PostgresProvider,
    migrations: &[MigrationDirectory],
    namespaces: Option<Namespaces>,
    external_shadow_db: UsingExternalShadowDb,
) -> ConnectorResult<SqlSchema> {
    let is_vanilla_postgres = !connector.is_cockroachdb();

    match external_shadow_db {
        UsingExternalShadowDb::Yes => {
            sql_schema_from_migration_history_for_external_db(connector, migrations, namespaces).await
        }

        // If we're not using an external shadow database, one must be created manually.
        UsingExternalShadowDb::No => {
            let (main_connection, params) = super::get_connection_and_params(&mut connector.state, provider).await?;

            let mut shadow_database_url: Url = params
                .connector_params
                .connection_string
                .parse()
                .map_err(ConnectorError::url_parse_error)?;

            let is_ppg = shadow_database_url.scheme() == MigratePostgresUrl::PRISMA_POSTGRES_SCHEME;

            if is_ppg && is_url_localhost(&shadow_database_url) {
                // when we're using a local PPG, we expect an external shadow database to be
                // provided via the URL
                return sql_schema_from_migration_history_for_local_ppg(
                    &shadow_database_url,
                    params.connector_params.preview_features,
                    migrations,
                    namespaces,
                )
                .await;
            }

            let shadow_database_name = crate::new_shadow_database_name();

            {
                let create_database = format!("CREATE DATABASE \"{shadow_database_name}\"");
                main_connection
                    .raw_cmd(&create_database)
                    .await
                    .map_err(|err| super::quaint_error_mapper(params)(err).into_shadow_db_creation_error())?;
            }

            if is_ppg {
                shadow_database_url
                    .query_pairs_mut()
                    .append_pair(PpgParams::DB_NAME_PARAM, &shadow_database_name);
            } else {
                shadow_database_url.set_path(&format!("/{shadow_database_name}"));
            }

            let preview_features = params.connector_params.preview_features;
            let connector_params = ConnectorParams::new(shadow_database_url.to_string(), preview_features, None);
            let mut shadow_database = PostgresConnector::new_with_params(connector_params)?;
            tracing::debug!("Connecting to shadow database `{}`", shadow_database_name);
            shadow_database.ensure_connection_validity().await?;

            // We go through the whole process without early return, then clean up
            // the shadow database, and only then return the result. This avoids
            // leaving shadow databases behind in case of e.g. faulty migrations.
            let circumstances = shadow_database.circumstances();
            let ret = shadow_database
                .with_connection(|conn, params| {
                    sql_schema_from_migrations_and_db(
                        conn,
                        params,
                        params.url.schema().to_owned(),
                        migrations,
                        namespaces,
                        circumstances,
                        params.connector_params.preview_features,
                    )
                })
                .await;

            shadow_database.dispose().await?;

            // if we don't drop the database, subsequent DROP DATABASE commands will fail
            drop(shadow_database);

            if is_vanilla_postgres {
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

async fn sql_schema_from_migration_history_for_external_db(
    connector: &mut PostgresConnector,
    migrations: &[MigrationDirectory],
    namespaces: Option<Namespaces>,
) -> Result<SqlSchema, ConnectorError> {
    connector.ensure_connection_validity().await?;
    tracing::info!("Connected to an external shadow database.");

    if connector.reset(namespaces.clone()).await.is_err() {
        crate::best_effort_reset(connector, namespaces.clone()).await?;
    }

    let circumstances = connector.circumstances();
    connector
        .with_connection(|conn, params| {
            sql_schema_from_migrations_and_db(
                conn,
                params,
                params.url.schema().to_owned(),
                migrations,
                namespaces,
                circumstances,
                params.connector_params.preview_features,
            )
        })
        .await
}

async fn sql_schema_from_migration_history_for_local_ppg(
    url: &Url,
    preview_features: PreviewFeatures,
    migrations: &[MigrationDirectory],
    namespaces: Option<Namespaces>,
) -> Result<SqlSchema, ConnectorError> {
    let ppg_params = PpgParams::parse_from(url)?;
    let shadow_db_url = ppg_params.local_shadow_database_url()?;

    let connector_params = ConnectorParams::new(shadow_db_url.to_string(), preview_features, None);
    let mut shadow_database = PostgresConnector::new_with_params(connector_params)?;

    let result = sql_schema_from_migration_history_for_external_db(&mut shadow_database, migrations, namespaces).await;

    shadow_database.dispose().await?;

    result
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
