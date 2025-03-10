#[cfg(feature = "sqlite-native")]
mod native;

#[cfg(not(feature = "sqlite-native"))]
mod wasm;

use std::future::Future;

#[cfg(feature = "sqlite-native")]
use native as imp;

#[cfg(not(feature = "sqlite-native"))]
use wasm as imp;

use crate::flavour::SqlFlavour;
use indoc::indoc;
use schema_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorResult, Namespaces,
};
use sql_schema_describer::{sqlite::SqlSchemaDescriber, DescriberErrorKind, SqlSchema};

type State = imp::State;

pub(crate) struct SqliteFlavour {
    state: State,
}

impl SqliteFlavour {
    fn with_connection<'a, F, O, C>(&'a mut self, f: C) -> BoxFuture<'a, ConnectorResult<O>>
    where
        O: 'a + Send,
        C: (FnOnce(&'a imp::Connection, &'a imp::Params) -> F) + Send + Sync + 'a,
        F: Future<Output = ConnectorResult<O>> + Send + 'a,
    {
        Box::pin(async move {
            let (connection, params) = imp::get_connection_and_params(&mut self.state)?;
            f(connection, params).await
        })
    }
}

#[cfg(feature = "sqlite-native")]
impl Default for SqliteFlavour {
    fn default() -> Self {
        Self { state: State::Initial }
    }
}

impl SqliteFlavour {
    #[cfg(not(feature = "sqlite-native"))]
    pub(crate) fn new_external(adapter: std::sync::Arc<dyn quaint::connector::ExternalConnector>) -> Self {
        Self {
            state: State::new(adapter, Default::default()),
        }
    }

    #[cfg(feature = "sqlite-native")]
    pub fn new_with_params(params: schema_connector::ConnectorParams) -> ConnectorResult<Self> {
        Ok(Self {
            state: State::WithParams(imp::Params::new(params)?),
        })
    }
}

impl std::fmt::Debug for SqliteFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<SQLite connector>")
    }
}

impl SqlFlavour for SqliteFlavour {
    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd("PRAGMA main.locking_mode=EXCLUSIVE")
    }

    fn connector_type(&self) -> &'static str {
        "sqlite"
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        self.with_connection(|conn, _| conn.apply_migration_script(migration_name, script))
    }

    fn table_names(&mut self, _namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<Vec<String>>> {
        Box::pin(async move {
            let select = r#"SELECT name AS table_name FROM sqlite_master WHERE type='table' ORDER BY name ASC"#;
            let rows = self.query_raw(select, &[]).await?;

            let table_names: Vec<String> = rows
                .into_iter()
                .flat_map(|row| row.get("table_name").and_then(|s| s.to_string()))
                .collect();

            Ok(table_names)
        })
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(imp::create_database(&self.state))
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(imp::drop_database(&self.state))
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(imp::ensure_connection_validity(&mut self.state))
    }

    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let sql = indoc! {r#"
            CREATE TABLE "_prisma_migrations" (
                "id"                    TEXT PRIMARY KEY NOT NULL,
                "checksum"              TEXT NOT NULL,
                "finished_at"           DATETIME,
                "migration_name"        TEXT NOT NULL,
                "logs"                  TEXT,
                "rolled_back_at"        DATETIME,
                "started_at"            DATETIME NOT NULL DEFAULT current_timestamp,
                "applied_steps_count"   INTEGER UNSIGNED NOT NULL DEFAULT 0
            );
        "#};

        self.raw_cmd(sql)
    }

    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::SQLITE
    }

    fn describe_schema(&mut self, _namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        self.with_connection(|conn, _| describe_schema(conn))
    }

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd("DROP TABLE _prisma_migrations")
    }

    fn load_migrations_table(
        &mut self,
    ) -> BoxFuture<
        '_,
        ConnectorResult<
            Result<Vec<schema_connector::MigrationRecord>, schema_connector::PersistenceNotInitializedError>,
        >,
    > {
        const SQL: &str = indoc! {r#"
            SELECT
                id,
                checksum,
                finished_at,
                migration_name,
                logs,
                rolled_back_at,
                started_at,
                applied_steps_count
            FROM `_prisma_migrations`
            ORDER BY `started_at` ASC
        "#};
        self.with_connection(|conn, _| async {
            let rows = match conn.query_raw(SQL, &[]).await {
                Ok(result) => result,
                Err(err) => {
                    #[cfg(feature = "sqlite-native")]
                    if let Some(native::rusqlite::Error::SqliteFailure(
                        native::rusqlite::ffi::Error {
                            extended_code: 1, // table not found
                            ..
                        },
                        _,
                    )) = err.source_as::<native::rusqlite::Error>()
                    {
                        return Ok(Err(schema_connector::PersistenceNotInitializedError));
                    }
                    return Err(err);
                }
            };

            let rows = rows
                .into_iter()
                .map(|row| -> ConnectorResult<_> {
                    Ok(schema_connector::MigrationRecord {
                        id: row.get("id").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg("Failed to extract `id` from `_prisma_migrations` row.".into())
                        })?,
                        checksum: row.get("checksum").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `checksum` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        finished_at: row.get("finished_at").and_then(|v| v.as_datetime()),
                        migration_name: row.get("migration_name").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `migration_name` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        logs: None,
                        rolled_back_at: row.get("rolled_back_at").and_then(|v| v.as_datetime()),
                        started_at: row.get("started_at").and_then(|v| v.as_datetime()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `started_at` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        applied_steps_count: row.get("applied_steps_count").and_then(|v| v.as_integer()).ok_or_else(
                            || {
                                ConnectorError::from_msg(
                                    "Failed to extract `applied_steps_count` from `_prisma_migrations` row.".into(),
                                )
                            },
                        )? as u32,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            tracing::debug!("Found {} migrations in the migrations table.", rows.len());

            Ok(Ok(rows))
        })
    }

    fn query<'a>(
        &'a mut self,
        query: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        self.with_connection(|conn, _| conn.query(query))
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        tracing::debug!(sql, params = ?params, query_type = "query_raw");
        self.with_connection(|conn, _| conn.query_raw(sql, params))
    }

    fn describe_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<quaint::connector::DescribedQuery>> {
        tracing::debug!(sql, query_type = "describe_query");
        self.with_connection(|conn, params| conn.describe_query(sql, params))
    }

    fn introspect(
        &mut self,
        _namespaces: Option<Namespaces>,
        _ctx: &schema_connector::IntrospectionContext,
    ) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        Box::pin(imp::introspect(&mut self.state))
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        self.with_connection(|conn, _| conn.raw_cmd(sql))
    }

    fn reset(&mut self, _namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        self.with_connection(|conn, params| conn.reset(params))
    }

    fn set_preview_features(&mut self, preview_features: enumflags2::BitFlags<psl::PreviewFeature>) {
        imp::set_preview_features(&mut self.state, preview_features)
    }

    #[tracing::instrument(skip(self, migrations))]
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a [MigrationDirectory],
        _shadow_database_connection_string: Option<String>,
        _namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        Box::pin(async move {
            tracing::debug!("Applying migrations to temporary in-memory SQLite database.");
            let shadow_db_conn = imp::Connection::new_in_memory();
            for migration in migrations {
                let script = migration.read_migration_script()?;

                tracing::debug!(
                    "Applying migration `{}` to shadow database.",
                    migration.migration_name()
                );

                shadow_db_conn.raw_cmd(&script).await.map_err(|connector_error| {
                    connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                })?;
            }

            describe_schema(&shadow_db_conn).await
        })
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        self.with_connection(|conn, _| conn.version())
    }

    fn search_path(&self) -> &str {
        "main"
    }
}

async fn acquire_lock(connection: &imp::Connection) -> ConnectorResult<()> {
    connection.raw_cmd("PRAGMA main.locking_mode=EXCLUSIVE").await
}

async fn describe_schema(connection: &imp::Connection) -> ConnectorResult<SqlSchema> {
    SqlSchemaDescriber::new(connection.as_connector())
        .describe_impl()
        .await
        .map_err(|err| match err.into_kind() {
            DescriberErrorKind::QuaintError(err) => ConnectorError::from_source(err, "Error describing the database."),
            DescriberErrorKind::CrossSchemaReference { .. } => {
                unreachable!("No schemas on SQLite")
            }
        })
}

fn ready<O: Send + Sync + 'static>(output: O) -> BoxFuture<'static, O> {
    Box::pin(std::future::ready(output))
}
