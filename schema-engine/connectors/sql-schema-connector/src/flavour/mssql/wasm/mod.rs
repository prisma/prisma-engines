//! All the quaint-wrangling for the mssql connector should happen here.

pub(super) mod shadow_db;

use enumflags2::BitFlags;
use quaint::connector::{ColumnType, DescribedColumn, DescribedParameter, GetRow, MssqlUrl, ToColumnNames};
use schema_connector::{BoxFuture, ConnectorError, ConnectorResult, Namespaces};
use sql_schema_describer::{mssql as describer, DescriberErrorKind, SqlSchema};
use user_facing_errors::schema_engine::ApplyMigrationError;

// TODO: use ExternalConnector here.
pub(super) struct Connection();

impl Connection {
    pub(super) async fn new(connection_str: &str) -> ConnectorResult<Self> {
        panic!("[sql-schema-connector::flavour::mssql::wasm] Not implemented");
    }

    #[tracing::instrument(skip(self, params))]
    pub(super) async fn describe_schema(
        &mut self,
        params: &super::Params,
        namespaces: Option<Namespaces>,
    ) -> ConnectorResult<SqlSchema> {
        panic!("[sql-schema-connector::flavour::mssql::wasm] Not implemented");
    }

    pub(super) async fn raw_cmd(&mut self, sql: &str, params: &super::Params) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        panic!("[sql-schema-connector::flavour::mssql::wasm] Not implemented");
    }

    pub(super) async fn version(&mut self, params: &super::Params) -> ConnectorResult<Option<String>> {
        tracing::debug!(query_type = "version");
        panic!("[sql-schema-connector::flavour::mssql::wasm] Not implemented");
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
        &mut self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
        conn_params: &super::Params,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql);
        panic!("[sql-schema-connector::flavour::mssql::wasm] Not implemented");
    }
}

pub(super) async fn generic_apply_migration_script(
    migration_name: &str,
    script: &str,
    conn: &mut Connection,
) -> ConnectorResult<()> {
    tracing::debug!(query_type = "raw_cmd", script);
    panic!("[sql-schema-connector::flavour::mssql::wasm] Not implemented");
}
