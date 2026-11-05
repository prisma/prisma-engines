mod native;

use super::KingbaseOracleDialect;
use crate::flavour::{SqlConnector, SqlDialect, State, UsingExternalShadowDb};
use indoc::indoc;
use quaint::connector::{DEFAULT_KINGBASE_ORACLE_SCHEMA, KingbaseOracleUrl};
use schema_connector::{
    BoxFuture, ConnectorError, ConnectorParams, ConnectorResult, Namespaces, SchemaFilter,
    migrations_directory::Migrations,
};
use sql_schema_describer::SqlSchema;
use std::{future, time::Duration};
use url::Url;

use native::Connection;

type ConnectorState = State<Params, Connection>;

const ADVISORY_LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const ADVISORY_LOCK_KEY: i64 = 72707369;

#[derive(Clone)]
pub(super) struct Params {
    connector_params: ConnectorParams,
    url: KingbaseOracleUrl,
    raw_url: Url,
    schema_name: String,
}

impl Params {
    fn new(connector_params: ConnectorParams) -> ConnectorResult<Self> {
        if let Some(shadow_db_url) = &connector_params.shadow_database_connection_string {
            super::super::validate_connection_infos_do_not_match(&connector_params.connection_string, shadow_db_url)?;
        }

        let mut raw_url: Url = connector_params
            .connection_string
            .parse()
            .map_err(ConnectorError::url_parse_error)?;
        // Startup search_path values are names, not SQL identifiers parsed by
        // CREATE SCHEMA. Use the physical spelling in the connection as well.
        if raw_url.query_pairs().any(|(key, _)| key == "schema") {
            let pairs = raw_url
                .query_pairs()
                .map(|(key, value)| {
                    let value = if key == "schema" {
                        sql_schema_describer::kingbase_oracle::normalize_namespace(&value).into_owned()
                    } else {
                        value.into_owned()
                    };
                    (key.into_owned(), value)
                })
                .collect::<Vec<_>>();
            raw_url.query_pairs_mut().clear().extend_pairs(pairs);
        }
        let url = KingbaseOracleUrl::new(raw_url.clone()).map_err(ConnectorError::url_parse_error)?;
        let schema_name = sql_schema_describer::kingbase_oracle::normalize_namespace(
            url.schema().unwrap_or(DEFAULT_KINGBASE_ORACLE_SCHEMA),
        )
        .into_owned();

        Ok(Self {
            connector_params,
            url,
            raw_url,
            schema_name,
        })
    }

    fn database_name(&self) -> &str {
        self.url
            .dbname()
            .unwrap_or(quaint::connector::DEFAULT_KINGBASE_ORACLE_DB)
    }

    fn schema_name(&self) -> &str {
        &self.schema_name
    }
}

/// Schema Engine connector for KingbaseES Oracle compatibility mode.
///
/// The database uses the PostgreSQL wire protocol, but its public SQL and
/// native types are Oracle-compatible. This connector deliberately keeps its
/// own URL, catalog describer and renderer rather than routing through the
/// PostgreSQL or Kingbase MySQL connectors.
pub(crate) struct KingbaseOracleConnector {
    state: ConnectorState,
}

impl std::fmt::Debug for KingbaseOracleConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KingbaseOracleConnector").finish()
    }
}

impl KingbaseOracleConnector {
    pub(crate) fn new_with_params(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(Self {
            state: State::WithParams(Params::new(params)?),
        })
    }

    fn schema_name(&self) -> &str {
        self.state
            .params()
            .map(Params::schema_name)
            .unwrap_or(DEFAULT_KINGBASE_ORACLE_SCHEMA)
    }
}

impl SqlConnector for KingbaseOracleConnector {
    fn dialect(&self) -> Box<dyn SqlDialect> {
        Box::new(KingbaseOracleDialect)
    }

    fn shadow_db_url(&self) -> Option<&str> {
        self.state
            .params()?
            .connector_params
            .shadow_database_connection_string
            .as_deref()
    }

    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(&mut self.state, |_params, connection| async move {
            crosstarget_utils::time::timeout(
                ADVISORY_LOCK_TIMEOUT,
                connection.raw_cmd(&format!("SELECT sys_catalog.sys_advisory_lock({ADVISORY_LOCK_KEY})")),
            )
            .await
            .map_err(|_| {
                ConnectorError::user_facing(user_facing_errors::common::DatabaseTimeout {
                    context: format!(
                        "Timed out trying to acquire a Kingbase Oracle advisory lock (SELECT sys_catalog.sys_advisory_lock({ADVISORY_LOCK_KEY})). Timeout: {}ms. See https://pris.ly/d/migrate-advisory-locking for details.",
                        ADVISORY_LOCK_TIMEOUT.as_millis()
                    ),
                })
            })??;

            Ok(())
        })
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, move |_params, connection| async move {
            connection.apply_migration_script(migration_name, script).await
        })
    }

    fn connector_type(&self) -> &'static str {
        "kingbase-oracle"
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        let params = self.state.get_unwrapped_params();
        let database_name = params.database_name().to_owned();
        let admin_url = maintenance_url(&params.raw_url, &database_name);

        Box::pin(async move {
            let mut connection = Connection::new(admin_url).await?;
            connection
                .raw_cmd(&format!("CREATE DATABASE {}", quote_identifier(&database_name)))
                .await?;
            Ok(database_name)
        })
    }

    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let sql = indoc! {r#"
            CREATE TABLE _prisma_migrations (
                id                      VARCHAR2(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR2(64) NOT NULL,
                finished_at             TIMESTAMP(3) WITH TIME ZONE,
                migration_name          VARCHAR2(255) NOT NULL,
                logs                    CLOB,
                rolled_back_at          TIMESTAMP(3) WITH TIME ZONE,
                started_at              TIMESTAMP(3) WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                applied_steps_count     INTEGER NOT NULL DEFAULT 0
            );
        "#};

        self.raw_cmd(sql)
    }

    fn describe_schema(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        with_connection(&mut self.state, |params, connection| async move {
            connection.describe_schema(params, namespaces).await
        })
    }

    fn introspect<'a>(
        &'a mut self,
        namespaces: Option<Namespaces>,
        _ctx: &'a schema_connector::IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        Box::pin(async move {
            let requested = Namespaces::to_vec(namespaces.clone(), self.schema_name().to_owned());
            let mut schema = self.describe_schema(namespaces).await?;
            sql_schema_describer::kingbase_oracle::preserve_namespace_spelling(&mut schema, &requested);
            Ok(schema)
        })
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let params = self.state.get_unwrapped_params().clone();
        // Kingbase refuses to drop a database while this connector owns a
        // session connected to it. Returning to the parameterized state drops
        // that session and keeps the connector reusable.
        self.state = State::WithParams(params.clone());
        let database_name = params.database_name().to_owned();
        let admin_url = maintenance_url(&params.raw_url, &database_name);

        Box::pin(async move {
            let mut connection = Connection::new(admin_url).await?;
            connection
                .raw_cmd(&format!("DROP DATABASE IF EXISTS {}", quote_identifier(&database_name)))
                .await
        })
    }

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd("DROP TABLE _prisma_migrations")
    }

    fn table_names(
        &mut self,
        namespaces: Option<Namespaces>,
        filters: SchemaFilter,
    ) -> BoxFuture<'_, ConnectorResult<Vec<String>>> {
        Box::pin(async move {
            // Keep the URL search-path schema visible in table-name lookups,
            // matching PostgreSQL. It contains migration bookkeeping even
            // when the Prisma schema lists only custom namespaces.
            let default_schema = self.schema_name().to_owned();
            let mut schema_names = Namespaces::to_vec(namespaces, default_schema.clone())
                .into_iter()
                .map(|name| sql_schema_describer::kingbase_oracle::normalize_namespace(&name).into_owned())
                .collect::<Vec<_>>();
            if !schema_names.iter().any(|schema| schema == &default_schema) {
                schema_names.push(default_schema);
            }
            schema_names.sort();
            schema_names.dedup();

            let mut table_names = Vec::new();

            for schema_name in schema_names {
                let rows = self
                    .query_raw(
                        "SELECT table_name FROM information_schema.tables WHERE table_schema = $1 AND table_type = 'BASE TABLE' ORDER BY table_name",
                        &[schema_name.clone().into()],
                    )
                    .await?;

                table_names.extend(
                    rows.into_iter()
                        .flat_map(|row| row.get("table_name").and_then(|value| value.to_string()))
                        .filter(|table_name| {
                            !self.dialect().schema_differ().contains_table(
                                &filters.external_tables,
                                Some(&schema_name),
                                table_name,
                            )
                        }),
                );
            }

            Ok(table_names)
        })
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(&mut self.state, |_params, _connection| future::ready(Ok(())))
    }

    fn query<'a>(
        &'a mut self,
        query: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(&mut self.state, move |_params, connection| async move {
            connection.query(query).await
        })
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(&mut self.state, move |_conn_params, connection| async move {
            connection.query_raw(sql, params).await
        })
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, move |_params, connection| async move {
            connection.raw_cmd(sql).await
        })
    }

    fn reset(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        let default_schema = self.schema_name().to_owned();

        with_connection(&mut self.state, move |_params, connection| async move {
            let mut schemas = namespaces
                .map(|namespaces| namespaces.into_iter().collect::<Vec<_>>())
                .unwrap_or_else(|| vec![default_schema])
                .into_iter()
                .map(|name| sql_schema_describer::kingbase_oracle::normalize_namespace(&name).into_owned())
                .collect::<Vec<_>>();
            schemas.sort();
            schemas.dedup();

            for schema in schemas {
                let schema = quote_identifier(&schema);
                connection.raw_cmd(&format!("DROP SCHEMA {schema} CASCADE")).await?;
                connection.raw_cmd(&format!("CREATE SCHEMA {schema}")).await?;
            }

            // This may already be gone with its schema. It is intentionally
            // best effort so a reset can still complete.
            let _ = connection.raw_cmd("DROP TABLE _prisma_migrations").await;
            Ok(())
        })
    }

    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
        external_shadow_db: UsingExternalShadowDb,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        match external_shadow_db {
            UsingExternalShadowDb::Yes => Box::pin(async move {
                self.ensure_connection_validity().await?;
                if self.reset(namespaces.clone()).await.is_err() {
                    crate::best_effort_reset(self, namespaces.clone(), filter).await?;
                }
                apply_migrations_and_describe(self, migrations, namespaces).await
            }),
            UsingExternalShadowDb::No => {
                let shadow_database_name = crate::new_shadow_database_name();

                with_connection(&mut self.state, move |params, connection| async move {
                    let create_database = format!("CREATE DATABASE {}", quote_identifier(&shadow_database_name));
                    connection
                        .raw_cmd(&create_database)
                        .await
                        .map_err(|err| err.into_shadow_db_creation_error())?;

                    let mut shadow_database_url = params.raw_url.clone();
                    shadow_database_url.set_path(&format!("/{shadow_database_name}"));
                    let shadow_params = ConnectorParams::new(
                        shadow_database_url.to_string(),
                        params.connector_params.preview_features,
                        None,
                    );
                    let mut shadow_database = KingbaseOracleConnector::new_with_params(shadow_params)?;
                    tracing::debug!("Connecting to Kingbase Oracle shadow database `{shadow_database_name}`");

                    let result = async {
                        shadow_database.ensure_connection_validity().await?;
                        if let Some(schema_name) = params.url.schema() {
                            shadow_database
                                .raw_cmd(&format!(
                                    "CREATE SCHEMA IF NOT EXISTS {}",
                                    quote_identifier(schema_name)
                                ))
                                .await?;
                        }
                        apply_migrations_and_describe(&mut shadow_database, migrations, namespaces).await
                    }
                    .await;

                    drop(shadow_database);

                    let drop_database = format!("DROP DATABASE IF EXISTS {}", quote_identifier(&shadow_database_name));
                    connection.raw_cmd(&drop_database).await?;

                    result
                })
            }
        }
    }

    fn set_preview_features(&mut self, preview_features: psl::PreviewFeatures) {
        match &mut self.state {
            State::Initial => {}
            State::WithParams(params) | State::Connected(params, _) => {
                params.connector_params.preview_features = preview_features;
            }
        }
    }

    fn preview_features(&self) -> psl::PreviewFeatures {
        self.state
            .params()
            .map(|params| params.connector_params.preview_features)
            .unwrap_or_default()
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        with_connection(&mut self.state, |_params, connection| async move {
            connection.version().await
        })
    }

    fn search_path(&self) -> &str {
        self.schema_name()
    }

    fn default_namespace(&self) -> Option<&str> {
        Some(self.schema_name())
    }

    fn describe_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<quaint::connector::DescribedQuery>> {
        with_connection(&mut self.state, move |_params, connection| async move {
            connection.describe_query(sql).await
        })
    }

    fn dispose(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

async fn apply_migrations_and_describe(
    connector: &mut KingbaseOracleConnector,
    migrations: &Migrations,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    if !migrations.shadow_db_init_script.trim().is_empty() {
        connector.raw_cmd(&migrations.shadow_db_init_script).await?;
    }

    for migration in migrations.migration_directories.iter() {
        let script = migration.read_migration_script()?;

        tracing::debug!(
            "Applying migration `{}` to Kingbase Oracle shadow database.",
            migration.migration_name()
        );

        connector
            .apply_migration_script(migration.migration_name(), &script)
            .await
            .map_err(|error| error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned()))?;
    }

    connector.describe_schema(namespaces).await
}

fn maintenance_url(url: &Url, database_name: &str) -> Url {
    let mut maintenance_url = url.clone();
    let maintenance_database = if database_name.eq_ignore_ascii_case("test") {
        "template1"
    } else {
        "test"
    };
    maintenance_url.set_path(&format!("/{maintenance_database}"));
    let query = url
        .query_pairs()
        .filter(|(key, _)| key != "schema")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    maintenance_url
        .query_pairs_mut()
        .clear()
        .extend_pairs(query.iter().map(|(key, value)| (key.as_str(), value.as_str())));
    maintenance_url
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('\"', "\"\""))
}

fn with_connection<'a, O, F, C>(state: &'a mut ConnectorState, f: C) -> BoxFuture<'a, ConnectorResult<O>>
where
    O: 'a,
    F: future::Future<Output = ConnectorResult<O>> + Send + 'a,
    C: FnOnce(&'a mut Params, &'a mut Connection) -> F + Send + 'a,
{
    match state {
        State::Initial => panic!("logic error: Initial"),
        State::Connected(params, connection) => Box::pin(f(params, connection)),
        state @ State::WithParams(_) => Box::pin(async move {
            state
                .try_connect(|params| Box::pin(async move { Connection::new(params.raw_url.clone()).await }))
                .await?;
            with_connection(state, f).await
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_url_uses_physical_name_without_changing_other_parameters() {
        for (requested, expected) in [("APP", "app"), ("app", "app"), ("App", "App")] {
            let params = Params::new(ConnectorParams {
                connection_string: format!(
                    "kingbase-oracle://localhost/test?schema={requested}&application_name=KeepCase"
                ),
                preview_features: Default::default(),
                shadow_database_connection_string: None,
            })
            .unwrap();
            assert_eq!(params.schema_name(), expected);
            assert_eq!(params.url.schema(), Some(expected));
            assert!(
                params
                    .raw_url
                    .query_pairs()
                    .any(|(key, value)| key == "schema" && value == expected)
            );
            assert!(
                params
                    .raw_url
                    .query_pairs()
                    .any(|(key, value)| key == "application_name" && value == "KeepCase")
            );
        }
    }
}
