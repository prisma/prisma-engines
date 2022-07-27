//! All the quaint-wrangling for the mssql connector should happen here.

use migration_connector::{ConnectorError, ConnectorResult};
use quaint::{
    connector::{self, MssqlUrl},
    prelude::{ConnectionInfo, Queryable},
};
use sql_schema_describer::{mssql as describer, DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use user_facing_errors::{
    introspection_engine::DatabaseSchemaInconsistent, migration_engine::ApplyMigrationError, KnownError,
};

pub(super) struct Connection(connector::Mssql);

impl Connection {
    pub(super) async fn new(connection_str: &str) -> ConnectorResult<Connection> {
        let url = MssqlUrl::new(connection_str).map_err(|err| {
            ConnectorError::user_facing(user_facing_errors::common::InvalidConnectionString {
                details: err.to_string(),
            })
        })?;
        Ok(Connection(
            connector::Mssql::new(url.clone()).await.map_err(quaint_err_url(&url))?,
        ))
    }

    #[tracing::instrument(skip(self, params))]
    pub(super) async fn describe_schema(&mut self, params: &super::Params) -> ConnectorResult<SqlSchema> {
        let mut schema = describer::SqlSchemaDescriber::new(&self.0)
            .describe(params.url.schema())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => quaint_err_url(&params.url)(err),
                e @ DescriberErrorKind::CrossSchemaReference { .. } => {
                    let err = KnownError::new(DatabaseSchemaInconsistent {
                        explanation: e.to_string(),
                    });

                    ConnectorError::from(err)
                }
            })?;

        crate::flavour::normalize_sql_schema(&mut schema, params.connector_params.preview_features);

        Ok(schema)
    }

    pub(super) async fn raw_cmd(&mut self, sql: &str, params: &super::Params) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.raw_cmd(sql).await.map_err(quaint_err(params))
    }

    pub(super) async fn version(&mut self, params: &super::Params) -> ConnectorResult<Option<String>> {
        tracing::debug!(query_type = "version");
        self.0.version().await.map_err(quaint_err(params))
    }

    pub(super) async fn query(
        &mut self,
        query: quaint::ast::Query<'_>,
        conn_params: &super::Params,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Mssql::build(query).unwrap();
        self.query_raw(&sql, &params, conn_params).await
    }

    pub(super) async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
        conn_params: &super::Params,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql, ?params);
        self.0.query_raw(sql, params).await.map_err(quaint_err(conn_params))
    }
}

pub(super) async fn generic_apply_migration_script(
    migration_name: &str,
    script: &str,
    conn: &mut Connection,
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
    quaint_err_url(&params.url)
}

fn quaint_err_url(url: &MssqlUrl) -> impl (Fn(quaint::error::Error) -> ConnectorError) + '_ {
    |err| crate::flavour::quaint_error_to_connector_error(err, &ConnectionInfo::Mssql(url.clone()))
}
