//! All the quaint-wrangling for the sqlite connector should happen here.

pub(crate) use quaint::connector::rusqlite;

use quaint::connector::{ColumnType, DescribedColumn, DescribedParameter, GetRow, ToColumnNames};
use schema_connector::{BoxFuture, ConnectorError, ConnectorParams, ConnectorResult};
use sql_schema_describer::SqlSchema;
use sqlx_core::{column::Column, type_info::TypeInfo};
use sqlx_sqlite::SqliteColumn;
use std::sync::Mutex;
use user_facing_errors::schema_engine::ApplyMigrationError;

use super::Params;

pub(super) type State = crate::flavour::State<Params, Connection>;

pub(super) struct Connection(Mutex<rusqlite::Connection>);

impl Connection {
    pub fn new(params: &super::Params) -> ConnectorResult<Self> {
        Ok(Connection(Mutex::new(
            rusqlite::Connection::open(&params.file_path).map_err(convert_error)?,
        )))
    }

    pub fn as_connector(&self) -> &Mutex<rusqlite::Connection> {
        &self.0
    }

    pub fn new_in_memory() -> Self {
        Connection(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()))
    }

    pub async fn raw_cmd(&self, sql: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        let conn = self.0.lock().unwrap();
        conn.execute_batch(sql).map_err(convert_error)
    }

    pub async fn query(&self, query: quaint::ast::Query<'_>) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Sqlite::build(query).unwrap();
        self.query_raw(&sql, &params).await
    }

    pub async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql);
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare_cached(sql).map_err(convert_error)?;

        let column_types = stmt.columns().iter().map(ColumnType::from).collect::<Vec<_>>();
        let mut rows = stmt
            .query(rusqlite::params_from_iter(params.iter()))
            .map_err(convert_error)?;
        let column_names = rows.to_column_names();
        let mut converted_rows = Vec::new();
        while let Some(row) = rows.next().map_err(convert_error)? {
            converted_rows.push(row.get_result_row().unwrap());
        }

        Ok(quaint::prelude::ResultSet::new(
            column_names,
            column_types,
            converted_rows,
        ))
    }

    pub async fn apply_migration_script(&self, migration_name: &str, script: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql = script);
        let conn = self.0.lock().unwrap();
        conn.execute_batch(script).map_err(|sqlite_error: rusqlite::Error| {
            let database_error_code = match sqlite_error {
                rusqlite::Error::SqliteFailure(rusqlite::ffi::Error { extended_code, .. }, _)
                | rusqlite::Error::SqlInputError {
                    error: rusqlite::ffi::Error { extended_code, .. },
                    ..
                } => extended_code.to_string(),
                _ => "none".to_owned(),
            };

            ConnectorError::user_facing(ApplyMigrationError {
                migration_name: migration_name.to_owned(),
                database_error_code,
                database_error: sqlite_error.to_string(),
            })
        })
    }

    pub(super) fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        super::ready(Ok(Some(quaint::connector::sqlite_version().to_owned())))
    }
}

pub(super) fn create_database(state: &State) -> ConnectorResult<String> {
    let params = state.get_unwrapped_params();
    let path = std::path::Path::new(&params.file_path);

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
}

pub(super) fn drop_database(state: &State) -> ConnectorResult<()> {
    let params = state.get_unwrapped_params();
    let file_path = &params.file_path;
    std::fs::remove_file(file_path)
        .map_err(|err| ConnectorError::from_msg(format!("Failed to delete SQLite database at `{file_path}`.\n{err}")))
}

pub(super) async fn ensure_connection_validity(state: &State) -> ConnectorResult<()> {
    let params = state.get_unwrapped_params();
    let path = std::path::Path::new(&params.file_path);
    // we use metadata() here instead of Path::exists() because we want accurate diagnostics:
    // if the file is not reachable because of missing permissions, we don't want to return
    // that the file doesn't exist.
    match std::fs::metadata(path) {
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
    }
}

pub(super) fn introspect(state: &mut State) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
    Box::pin(async move {
        let (conn, params) = get_connection_and_params(state)?;
        let path = std::path::Path::new(&params.file_path);
        if std::fs::metadata(path).is_err() {
            return Err(ConnectorError::user_facing(
                user_facing_errors::common::DatabaseDoesNotExist::Sqlite {
                    database_file_name: path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    database_file_path: params.file_path.clone(),
                },
            ));
        }

        super::describe_schema(conn).await
    })
}

pub(super) async fn reset(state: &mut State) -> ConnectorResult<()> {
    let (connection, params) = get_connection_and_params(state)?;
    let file_path = &params.file_path;

    connection.raw_cmd("PRAGMA main.locking_mode=NORMAL").await?;
    connection.raw_cmd("PRAGMA main.quick_check").await?;

    tracing::debug!("Truncating {:?}", file_path);

    std::fs::File::create(file_path).map_err(|io_error| {
        ConnectorError::from_source(
            io_error,
            "Failed to truncate sqlite file. Please check that you have write permissions on the directory.",
        )
    })?;

    super::acquire_lock(connection).await
}

pub(super) fn describe_query(state: &mut State, sql: &str) -> ConnectorResult<quaint::connector::DescribedQuery> {
    let (conn, params) = get_connection_and_params(state)?;
    tracing::debug!(query_type = "describe_query", sql);
    // SQLite only provides type information for _declared_ column types. That means any expression will not contain type information.
    // Sqlx works around this by running an `EXPLAIN` query and inferring types by interpreting sqlite bytecode.
    // If you're curious, here's the code: https://github.com/launchbadge/sqlx/blob/16e3f1025ad1e106d1acff05f591b8db62d688e2/sqlx-sqlite/src/connection/explain.rs#L557
    // We use SQLx's as a fallback for when quaint's infers Unknown.
    let describe = sqlx_sqlite::describe_blocking(sql, &params.file_path)
        .map_err(|err| ConnectorError::from_source(err, "Error describing the query."))?;
    let conn = conn.0.lock().unwrap();
    let stmt = conn.prepare_cached(sql).map_err(convert_error)?;

    let parameters = (1..=stmt.parameter_count())
        .map(|idx| match stmt.parameter_name(idx) {
            Some(name) => {
                // SQLite parameter names are prefixed with a colon. We remove it here so that the js doc parser can match the names.
                let name = name.strip_prefix(':').unwrap_or(name);

                DescribedParameter::new_named(name, ColumnType::Unknown)
            }
            None => DescribedParameter::new_unnamed(idx, ColumnType::Unknown),
        })
        .collect();
    let columns = stmt
        .columns()
        .iter()
        .zip(&describe.nullable)
        .enumerate()
        .map(|(idx, (col, nullable))| {
            let typ = match ColumnType::from(col) {
                // If the column type is unknown, we try to infer it from the describe.
                ColumnType::Unknown => describe.column(idx).to_column_type(),
                typ => typ,
            };

            DescribedColumn::new_named(col.name(), typ).is_nullable(nullable.unwrap_or(true))
        })
        .collect();

    Ok(quaint::connector::DescribedQuery {
        columns,
        parameters,
        enum_names: None,
    })
}

pub(super) fn connection_string(state: &State) -> Option<&str> {
    state
        .params()
        .map(|params| params.connector_params.connection_string.as_str())
}

pub(super) fn get_connection(state: &mut State) -> ConnectorResult<&mut Connection> {
    let (conn, _) = get_connection_and_params(state)?;
    Ok(conn)
}

fn get_connection_and_params(state: &mut State) -> ConnectorResult<(&mut Connection, &mut Params)> {
    match state {
        super::State::Initial => panic!("logic error: Initial"),
        super::State::Connected(params, conn) => Ok((conn, params)),
        super::State::WithParams(p) => {
            let conn = Connection::new(p)?;
            let params = match std::mem::replace(state, super::State::Initial) {
                super::State::WithParams(p) => p,
                _ => unreachable!(),
            };
            *state = super::State::Connected(params, conn);
            get_connection_and_params(state)
        }
    }
}

pub(super) fn set_preview_features(state: &mut State, preview_features: enumflags2::BitFlags<psl::PreviewFeature>) {
    match state {
        super::State::Initial => {
            if !preview_features.is_empty() {
                tracing::warn!("set_preview_feature on Initial state has no effect ({preview_features}).");
            }
        }
        super::State::WithParams(params) | super::State::Connected(params, _) => {
            params.connector_params.preview_features = preview_features
        }
    }
}

pub(super) fn set_params(state: &mut State, params: ConnectorParams) -> ConnectorResult<()> {
    let quaint::connector::SqliteParams { file_path, .. } =
        quaint::connector::SqliteParams::try_from(params.connection_string.as_str())
            .map_err(ConnectorError::url_parse_error)?;

    state.set_params(Params {
        connector_params: params,
        file_path,
    });
    Ok(())
}

fn convert_error(err: rusqlite::Error) -> ConnectorError {
    ConnectorError::from_source(err, "SQLite database error")
}

trait ToColumnTypeExt {
    fn to_column_type(&self) -> ColumnType;
}

impl ToColumnTypeExt for &SqliteColumn {
    fn to_column_type(&self) -> ColumnType {
        let ty = self.type_info();

        match ty.name() {
            "NULL" => ColumnType::Null,
            "TEXT" => ColumnType::Text,
            "REAL" => ColumnType::Double,
            "BLOB" => ColumnType::Bytes,
            "INTEGER" => ColumnType::Int64,
            // Not supported by sqlx-sqlite
            "NUMERIC" => ColumnType::Numeric,

            // non-standard extensions
            "BOOLEAN" => ColumnType::Boolean,
            "DATE" => ColumnType::Date,
            "TIME" => ColumnType::Time,
            "DATETIME" => ColumnType::DateTime,
            _ => ColumnType::Unknown,
        }
    }
}
