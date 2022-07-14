//! All the quaint-wrangling for the sqlite connector should happen here.

use migration_connector::{ConnectorError, ConnectorResult};
use quaint::{
    connector,
    prelude::{ConnectionInfo, Queryable},
};
use sql_schema_describer::{sqlite as describer, DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use user_facing_errors::migration_engine::ApplyMigrationError;

pub(super) struct Connection(connector::Sqlite);

impl Connection {
    pub(super) fn new(params: &super::Params) -> ConnectorResult<Self> {
        Ok(Connection(
            connector::Sqlite::new(&params.file_path).map_err(quaint_err(params))?,
        ))
    }

    pub(super) fn new_in_memory() -> Self {
        Connection(connector::Sqlite::new_in_memory().unwrap())
    }

    pub(super) async fn describe_schema(&mut self, params: &super::Params) -> ConnectorResult<SqlSchema> {
        describer::SqlSchemaDescriber::new(&self.0)
            .describe(&params.attached_name)
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => quaint_err(params)(err),
                DescriberErrorKind::CrossSchemaReference { .. } => {
                    unreachable!("No schemas on SQLite")
                }
            })
    }

    pub(super) async fn raw_cmd(&mut self, sql: &str, params: &super::Params) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.raw_cmd(sql).await.map_err(quaint_err(params))
    }

    pub(super) async fn query(
        &mut self,
        query: quaint::ast::Query<'_>,
        conn_params: &super::Params,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Sqlite::build(query).unwrap();
        self.query_raw(&sql, &params, conn_params).await
    }

    pub(super) async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
        conn_params: &super::Params,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.query_raw(sql, params).await.map_err(quaint_err(conn_params))
    }
}

pub(super) async fn generic_apply_migration_script(
    migration_name: &str,
    script: &str,
    conn: &Connection,
) -> ConnectorResult<()> {
    tracing::debug!(query_type = "raw_cmd", sql = script);
    conn.0.raw_cmd(script).await.map_err(|sql_error| {
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

fn quaint_err(params: &super::Params) -> impl (Fn(quaint::error::Error) -> ConnectorError) + '_ {
    |err| {
        super::super::quaint_error_to_connector_error(
            err,
            &ConnectionInfo::Sqlite {
                file_path: params.file_path.clone(),
                db_name: params.attached_name.clone(),
            },
        )
    }
}
