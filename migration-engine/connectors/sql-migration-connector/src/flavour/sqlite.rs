use crate::flavour::SqlFlavour;
use indoc::indoc;
use migration_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult,
};
use quaint::{
    connector::Sqlite as Connection,
    prelude::{ConnectionInfo, Queryable},
};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::{future, path::Path};
use user_facing_errors::migration_engine::ApplyMigrationError;

type State = super::State<Params, Connection>;

struct Params {
    connector_params: ConnectorParams,
    file_path: String,
    attached_name: String,
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

    fn run_query_script<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd(sql)
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, move |_params, connection| async move {
            generic_apply_migration_script(migration_name, script, connection).await
        })
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

            Connection::new(&params.file_path).map_err(quaint_err(params))?;

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

    fn datamodel_connector(&self) -> &'static dyn datamodel::datamodel_connector::Connector {
        datamodel::builtin_connectors::SQLITE
    }

    fn describe_schema(&mut self) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        use sql_schema_describer::{sqlite as describer, DescriberErrorKind};
        with_connection(&mut self.state, |params, connection| async move {
            describer::SqlSchemaDescriber::new(connection)
                .describe(&params.attached_name)
                .await
                .map_err(|err| match err.into_kind() {
                    DescriberErrorKind::QuaintError(err) => quaint_err(params)(err),
                    DescriberErrorKind::CrossSchemaReference { .. } => {
                        unreachable!("No schemas on SQLite")
                    }
                })
        })
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let params = self.state.get_unwrapped_params();
        let file_path = &params.file_path;
        let ret = std::fs::remove_file(file_path).map_err(|err| {
            ConnectorError::from_msg(format!("Failed to delete SQLite database at `{}`.\n{}", file_path, err))
        });
        Box::pin(std::future::ready(ret))
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

        Box::pin(future::ready(result))
    }

    fn query<'a>(
        &'a mut self,
        query: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(&mut self.state, |params, conn| async {
            conn.query(query).await.map_err(quaint_err(params))
        })
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(&mut self.state, |conn_params, conn: &'_ mut Connection| async {
            conn.query_raw(sql, params).await.map_err(quaint_err(conn_params))
        })
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, |params, conn| async {
            conn.raw_cmd(sql).await.map_err(quaint_err(params))
        })
    }

    fn reset(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(&mut self.state, move |params, connection| async move {
            let file_path = &params.file_path;

            raw_cmd("PRAGMA main.locking_mode=NORMAL", connection, params).await?;
            raw_cmd("PRAGMA main.quick_check", connection, params).await?;

            tracing::debug!("Truncating {:?}", file_path);
            std::fs::File::create(file_path).expect("failed to truncate sqlite file");

            acquire_lock(connection, params).await?;

            Ok(())
        })
    }

    fn set_params(&mut self, params: ConnectorParams) -> ConnectorResult<()> {
        let quaint::connector::SqliteParams { file_path, db_name, .. } =
            quaint::connector::SqliteParams::try_from(params.connection_string.as_str())
                .map_err(ConnectorError::url_parse_error)?;

        self.state.set_params(Params {
            connector_params: params,
            file_path,
            attached_name: db_name,
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
            let conn = Connection::new_in_memory().unwrap();
            let conn_info = ConnectionInfo::Sqlite {
                file_path: "<in_memory>".into(),
                db_name: "<in_memory>".into(),
            };

            for migration in migrations {
                let script = migration.read_migration_script()?;

                tracing::debug!(
                    "Applying migration `{}` to shadow database.",
                    migration.migration_name()
                );

                conn.raw_cmd(&script)
                    .await
                    .map_err(|err| super::quaint_error_to_connector_error(err, &conn_info))
                    .map_err(ConnectorError::from)
                    .map_err(|connector_error| {
                        connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                    })?;
            }

            let sql_schema = sql_schema_describer::sqlite::SqlSchemaDescriber::new(&conn)
                .describe("")
                .await
                .unwrap();

            Ok(sql_schema)
        })
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        Box::pin(future::ready(Ok(Some(quaint::connector::sqlite_version().to_owned()))))
    }
}

async fn acquire_lock(connection: &Connection, params: &Params) -> ConnectorResult<()> {
    connection
        .raw_cmd("PRAGMA main.locking_mode=EXCLUSIVE")
        .await
        .map_err(quaint_err(params))
}

fn with_connection<'a, O, F, C>(state: &'a mut State, f: C) -> BoxFuture<'a, ConnectorResult<O>>
where
    O: 'a,
    F: future::Future<Output = ConnectorResult<O>> + Send + 'a,
    C: (FnOnce(&'a mut Params, &'a mut Connection) -> F) + Send + 'a,
{
    match state {
        super::State::Initial => panic!("logic error: Initial"),
        super::State::Connected(p, c) => Box::pin(f(p, c)),
        state @ super::State::WithParams(_) => Box::pin(async move {
            state
                .try_connect(|params| {
                    Box::pin(std::future::ready(
                        Connection::new(&params.file_path).map_err(quaint_err(params)),
                    ))
                })
                .await?;
            with_connection(state, f).await
        }),
    }
}

async fn raw_cmd(sql: &str, connection: &Connection, params: &Params) -> ConnectorResult<()> {
    connection.raw_cmd(sql).await.map_err(quaint_err(params))
}

fn quaint_err(params: &Params) -> impl (Fn(quaint::error::Error) -> ConnectorError) + '_ {
    |err| {
        super::quaint_error_to_connector_error(
            err,
            &ConnectionInfo::Sqlite {
                file_path: params.file_path.clone(),
                db_name: params.attached_name.clone(),
            },
        )
    }
}

async fn generic_apply_migration_script(migration_name: &str, script: &str, conn: &Connection) -> ConnectorResult<()> {
    conn.raw_cmd(script).await.map_err(|sql_error| {
        ConnectorError::user_facing(ApplyMigrationError {
            migration_name: migration_name.to_owned(),
            database_error_code: String::from(sql_error.original_code().unwrap_or("none")),
            database_error: sql_error
                .original_message()
                .map(String::from)
                .unwrap_or_else(|| sql_error.to_string()),
        })
    })
}
