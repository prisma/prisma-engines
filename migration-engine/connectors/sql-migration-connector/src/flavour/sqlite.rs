mod connection;

use self::connection::*;
use crate::flavour::SqlFlavour;
use indoc::indoc;
use migration_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult,
};
use sql_schema_describer::SqlSchema;
use std::{future, path::Path};

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

    fn datamodel_connector(&self) -> &'static dyn datamodel::datamodel_connector::Connector {
        datamodel::builtin_connectors::SQLITE
    }

    fn describe_schema(&mut self) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        with_connection(&mut self.state, |params, connection| async move {
            connection.describe_schema(params).await
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
            conn.query(query, params).await
        })
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        tracing::debug!(sql, params = ?params, query_type = "query_raw");
        with_connection(&mut self.state, |conn_params, conn: &mut Connection| async {
            conn.query_raw(sql, params, conn_params).await
        })
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, |params, conn| async {
            conn.raw_cmd(sql, params).await
        })
    }

    fn reset(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(&mut self.state, move |params, connection| async move {
            let file_path = &params.file_path;

            connection.raw_cmd("PRAGMA main.locking_mode=NORMAL", params).await?;
            connection.raw_cmd("PRAGMA main.quick_check", params).await?;

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
            let mut shadow_db_conn = Connection::new_in_memory();
            let shadow_db_params = Params {
                connector_params: ConnectorParams {
                    connection_string: String::new(),
                    preview_features: Default::default(),
                    shadow_database_connection_string: None,
                },
                file_path: String::new(),
                attached_name: String::new(),
            };

            for migration in migrations {
                let script = migration.read_migration_script()?;

                tracing::debug!(
                    "Applying migration `{}` to shadow database.",
                    migration.migration_name()
                );

                shadow_db_conn
                    .raw_cmd(&script, &shadow_db_params)
                    .await
                    .map_err(|connector_error| {
                        connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                    })?;
            }

            let sql_schema = shadow_db_conn.describe_schema(&shadow_db_params).await?;

            Ok(sql_schema)
        })
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        Box::pin(future::ready(Ok(Some(quaint::connector::sqlite_version().to_owned()))))
    }
}

async fn acquire_lock(connection: &mut Connection, params: &Params) -> ConnectorResult<()> {
    connection.raw_cmd("PRAGMA main.locking_mode=EXCLUSIVE", params).await
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
                .try_connect(|params| Box::pin(std::future::ready(Connection::new(params))))
                .await?;
            with_connection(state, f).await
        }),
    }
}
