mod connection;

use self::connection::*;
use crate::flavour::SqlFlavour;
use indoc::indoc;
use migration_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult,
};
use sql_schema_describer::SqlSchema;
use std::path::Path;

type State = super::State<Params, Connection>;

struct Params {
    connector_params: ConnectorParams,
    file_path: String,
}

pub(crate) struct SqliteFlavour {
    state: State,
}

impl Default for SqliteFlavour {
    fn default() -> Self {
        SqliteFlavour { state: State::Initial }
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
        ready(with_connection(&mut self.state, move |_params, connection| {
            generic_apply_migration_script(migration_name, script, connection)
        }))
    }

    fn connection_string(&self) -> Option<&str> {
        self.state
            .params()
            .map(|p| p.connector_params.connection_string.as_str())
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(async {
            let params = self.state.get_unwrapped_params();
            let path = Path::new(&params.file_path);

            if path.exists() {
                return Ok(params.file_path.clone());
            }

            let dir = path.parent();

            if let Some((dir, false)) = dir.map(|dir| (dir, dir.exists())) {
                std::fs::create_dir_all(dir)
                    .map_err(|err| ConnectorError::from_source(err, "Creating SQLite database parent directory."))?;
            }

            Connection::new(params)?;

            Ok(params.file_path.clone())
        })
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

    fn describe_schema(&mut self) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        Box::pin(async move {
            let schema = with_connection(&mut self.state, |_, conn| Ok(Box::pin(conn.describe_schema())))?.await?;
            Ok(schema)
        })
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let params = self.state.get_unwrapped_params();
        let file_path = &params.file_path;
        let ret = std::fs::remove_file(file_path).map_err(|err| {
            ConnectorError::from_msg(format!("Failed to delete SQLite database at `{}`.\n{}", file_path, err))
        });
        ready(ret)
    }

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd("DROP TABLE _prisma_migrations")
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let params = self.state.get_unwrapped_params();
        let path = std::path::Path::new(&params.file_path);
        // we use metadata() here instead of Path::exists() because we want accurate diagnostics:
        // if the file is not reachable because of missing permissions, we don't want to return
        // that the file doesn't exist.
        let result = match std::fs::metadata(path) {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(ConnectorError::user_facing(
                user_facing_errors::common::DatabaseDoesNotExist::Sqlite {
                    database_file_name: path
                        .file_name()
                        .map(|osstr| osstr.to_string_lossy().into_owned())
                        .unwrap_or_else(|| params.file_path.clone()),
                    database_file_path: params.file_path.clone(),
                },
            )),
            Err(err) => Err(ConnectorError::from_source(err, "Failed to open SQLite database.")),
        };

        ready(result)
    }

    fn load_migrations_table(
        &mut self,
    ) -> BoxFuture<
        '_,
        ConnectorResult<
            Result<Vec<migration_connector::MigrationRecord>, migration_connector::PersistenceNotInitializedError>,
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
        ready(with_connection(&mut self.state, |_, conn| {
            let rows = match conn.query_raw(SQL, &[]) {
                Ok(result) => result,
                Err(err) => {
                    if let Some(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error {
                            extended_code: 1, // table not found
                            ..
                        },
                        _,
                    )) = err.source_as::<rusqlite::Error>()
                    {
                        return Ok(Err(migration_connector::PersistenceNotInitializedError));
                    } else {
                        return Err(err);
                    }
                }
            };

            let rows = rows
                .into_iter()
                .map(|row| -> ConnectorResult<_> {
                    Ok(migration_connector::MigrationRecord {
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
        }))
    }

    fn query<'a>(
        &'a mut self,
        query: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        ready(with_connection(&mut self.state, |_, conn| conn.query(query)))
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        tracing::debug!(sql, params = ?params, query_type = "query_raw");
        ready(with_connection(&mut self.state, |_, conn| conn.query_raw(sql, params)))
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        ready(with_connection(&mut self.state, |_, conn| conn.raw_cmd(sql)))
    }

    fn reset(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        ready(with_connection(&mut self.state, move |params, connection| {
            let file_path = &params.file_path;

            connection.raw_cmd("PRAGMA main.locking_mode=NORMAL")?;
            connection.raw_cmd("PRAGMA main.quick_check")?;

            tracing::debug!("Truncating {:?}", file_path);
            std::fs::File::create(file_path).expect("failed to truncate sqlite file");

            acquire_lock(connection)?;

            Ok(())
        }))
    }

    fn set_params(&mut self, params: ConnectorParams) -> ConnectorResult<()> {
        let quaint::connector::SqliteParams { file_path, .. } =
            quaint::connector::SqliteParams::try_from(params.connection_string.as_str())
                .map_err(ConnectorError::url_parse_error)?;

        self.state.set_params(Params {
            connector_params: params,
            file_path,
        });
        Ok(())
    }

    #[tracing::instrument(skip(self, migrations))]
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a [MigrationDirectory],
        _shadow_database_connection_string: Option<String>,
    ) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        Box::pin(async move {
            tracing::debug!("Applying migrations to temporary in-memory SQLite database.");
            let mut shadow_db_conn = Connection::new_in_memory();
            for migration in migrations {
                let script = migration.read_migration_script()?;

                tracing::debug!(
                    "Applying migration `{}` to shadow database.",
                    migration.migration_name()
                );

                shadow_db_conn.raw_cmd(&script).map_err(|connector_error| {
                    connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                })?;
            }

            shadow_db_conn.describe_schema().await
        })
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        ready(Ok(Some(quaint::connector::sqlite_version().to_owned())))
    }
}

fn acquire_lock(connection: &mut Connection) -> ConnectorResult<()> {
    connection.raw_cmd("PRAGMA main.locking_mode=EXCLUSIVE")
}

fn with_connection<'a, O, C>(state: &'a mut State, f: C) -> ConnectorResult<O>
where
    O: 'a + Send,
    C: (FnOnce(&'a mut Params, &'a mut Connection) -> ConnectorResult<O>) + Send + Sync + 'a,
{
    match state {
        super::State::Initial => panic!("logic error: Initial"),
        super::State::Connected(p, c) => f(p, c),
        super::State::WithParams(p) => {
            let conn = Connection::new(p)?;
            let params = match std::mem::replace(state, super::State::Initial) {
                super::State::WithParams(p) => p,
                _ => unreachable!(),
            };
            *state = super::State::Connected(params, conn);
            with_connection(state, f)
        }
    }
}

fn ready<O: Send + Sync + 'static>(output: O) -> BoxFuture<'static, O> {
    Box::pin(std::future::ready(output))
}
