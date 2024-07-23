//! All the quaint-wrangling for the sqlite connector should happen here.

pub(crate) use quaint::connector::rusqlite;

use quaint::connector::{ColumnType, GetRow, ToColumnNames};
use schema_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::{sqlite as describer, DescriberErrorKind, SqlSchema};
use std::sync::Mutex;
use user_facing_errors::schema_engine::ApplyMigrationError;

pub(super) struct Connection(Mutex<rusqlite::Connection>);

impl Connection {
    pub(super) fn new(params: &super::Params) -> ConnectorResult<Self> {
        Ok(Connection(Mutex::new(
            rusqlite::Connection::open(&params.file_path).map_err(convert_error)?,
        )))
    }

    pub(super) fn new_in_memory() -> Self {
        Connection(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()))
    }

    pub(super) async fn describe_schema(&mut self) -> ConnectorResult<SqlSchema> {
        describer::SqlSchemaDescriber::new(&self.0)
            .describe_impl()
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    ConnectorError::from_source(err, "Error describing the database.")
                }
                DescriberErrorKind::CrossSchemaReference { .. } => {
                    unreachable!("No schemas on SQLite")
                }
            })
    }

    pub(super) fn raw_cmd(&mut self, sql: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        let conn = self.0.lock().unwrap();
        conn.execute_batch(sql).map_err(convert_error)
    }

    pub(super) fn query(&mut self, query: quaint::ast::Query<'_>) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Sqlite::build(query).unwrap();
        self.query_raw(&sql, &params)
    }

    pub(super) fn query_raw(
        &mut self,
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
}

pub(super) fn generic_apply_migration_script(
    migration_name: &str,
    script: &str,
    conn: &Connection,
) -> ConnectorResult<()> {
    tracing::debug!(query_type = "raw_cmd", sql = script);
    let conn = conn.0.lock().unwrap();
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

fn convert_error(err: rusqlite::Error) -> ConnectorError {
    ConnectorError::from_source(err, "SQLite database error")
}
