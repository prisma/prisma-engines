#[cfg(feature = "kingbase-mysql-native")]
mod native;

use super::KingbaseMysqlDialect;
use crate::flavour::{SqlConnector, SqlDialect, State, UsingExternalShadowDb};
use indoc::indoc;
use quaint::connector::{DEFAULT_KINGBASE_MYSQL_SCHEMA, KingbaseMysqlUrl};
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
struct Params {
    connector_params: ConnectorParams,
    url: KingbaseMysqlUrl,
    raw_url: Url,
}

impl Params {
    fn new(connector_params: ConnectorParams) -> ConnectorResult<Self> {
        if let Some(shadow_db_url) = &connector_params.shadow_database_connection_string {
            super::super::validate_connection_infos_do_not_match(&connector_params.connection_string, shadow_db_url)?;
        }

        let raw_url: Url = connector_params
            .connection_string
            .parse()
            .map_err(ConnectorError::url_parse_error)?;
        let url = KingbaseMysqlUrl::new(raw_url.clone()).map_err(ConnectorError::url_parse_error)?;

        Ok(Self {
            connector_params,
            url,
            raw_url,
        })
    }

    fn database_name(&self) -> &str {
        self.url
            .dbname()
            .unwrap_or(quaint::connector::DEFAULT_KINGBASE_MYSQL_DB)
    }

    fn schema_name(&self) -> &str {
        self.url
            .schema()
            .unwrap_or(quaint::connector::DEFAULT_KINGBASE_MYSQL_SCHEMA)
    }
}

pub(crate) struct KingbaseMysqlConnector {
    state: ConnectorState,
}

impl std::fmt::Debug for KingbaseMysqlConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KingbaseMysqlConnector").finish()
    }
}

impl KingbaseMysqlConnector {
    pub(crate) fn new_with_params(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(Self {
            state: State::WithParams(Params::new(params)?),
        })
    }

    fn schema_name(&self) -> &str {
        self.state
            .params()
            .and_then(|params| params.url.schema())
            .unwrap_or(DEFAULT_KINGBASE_MYSQL_SCHEMA)
    }
}

impl SqlConnector for KingbaseMysqlConnector {
    fn dialect(&self) -> Box<dyn SqlDialect> {
        Box::new(KingbaseMysqlDialect::default())
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
                connection.raw_cmd(&format!("SELECT sys_advisory_lock({ADVISORY_LOCK_KEY})")),
            )
            .await
            .map_err(|_| {
                ConnectorError::user_facing(user_facing_errors::common::DatabaseTimeout {
                    context: format!(
                        "Timed out trying to acquire a Kingbase advisory lock (SELECT sys_advisory_lock({ADVISORY_LOCK_KEY})). Timeout: {}ms. See https://pris.ly/d/migrate-advisory-locking for details.",
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
        "kingbase-mysql"
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
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             DATETIME(3),
                migration_name          VARCHAR(255) NOT NULL,
                logs                    TEXT,
                rolled_back_at          DATETIME(3),
                started_at              DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
                applied_steps_count     INTEGER UNSIGNED NOT NULL DEFAULT 0
            );
        "#};

        self.raw_cmd(sql)
    }

    fn describe_schema(&mut self, _namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        with_connection(&mut self.state, |params, connection| async move {
            connection.describe_schema(params).await
        })
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let params = self.state.get_unwrapped_params().clone();
        // Kingbase refuses to drop a database while this connector still owns a
        // session connected to it. Dropping the connection also leaves the
        // connector reusable in the parameterized state.
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
        _namespaces: Option<Namespaces>,
        filters: SchemaFilter,
    ) -> BoxFuture<'_, ConnectorResult<Vec<String>>> {
        Box::pin(async move {
            let schema_name = self.schema_name().to_owned();
            let rows = self
                .query_raw(
                    "SELECT table_name FROM information_schema.tables WHERE table_schema = ? AND table_type = 'BASE TABLE' ORDER BY table_name",
                    &[schema_name.into()],
                )
                .await?;

            Ok(rows
                .into_iter()
                .flat_map(|row| row.get("table_name").and_then(|value| value.to_string()))
                .filter(|table_name| {
                    !self
                        .dialect()
                        .schema_differ()
                        .contains_table(&filters.external_tables, None, table_name)
                })
                .collect())
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

    fn reset(&mut self, _namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async {
            Err(ConnectorError::from_msg(
                "Kingbase MySQL reset requires a schema filter.".into(),
            ))
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
                crate::best_effort_reset(self, namespaces.clone(), filter).await?;
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
                    let mut shadow_database = KingbaseMysqlConnector::new_with_params(shadow_params)?;
                    tracing::debug!("Connecting to Kingbase shadow database `{shadow_database_name}`");

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
        None
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
    connector: &mut KingbaseMysqlConnector,
    migrations: &Migrations,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    if !migrations.shadow_db_init_script.trim().is_empty() {
        connector.raw_cmd(&migrations.shadow_db_init_script).await?;
    }

    for migration in migrations.migration_directories.iter() {
        let script = migration.read_migration_script()?;

        tracing::debug!(
            "Applying migration `{}` to Kingbase shadow database.",
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
    format!("`{}`", identifier.replace('`', "``"))
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
