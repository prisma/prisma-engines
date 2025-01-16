//! All the quaint-wrangling for the sqlite connector should happen here.

use quaint::connector::{ColumnType, DescribedColumn, DescribedParameter, GetRow, ToColumnNames};
use schema_connector::{BoxFuture, ConnectorError, ConnectorResult, Namespaces};
use sql_schema_describer::{sqlite as describer, DescriberErrorKind, SqlSchema};
use user_facing_errors::schema_engine::ApplyMigrationError;

// TODO: use ExternalConnector here.
pub(super) struct Connection();

impl Connection {
    pub(super) fn new(params: &super::Params) -> ConnectorResult<Self> {
        panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
    }

    pub(super) fn new_in_memory() -> Self {
        panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
    }

    pub(super) async fn describe_schema(&mut self) -> ConnectorResult<SqlSchema> {
        panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
    }

    pub(super) fn raw_cmd(&mut self, sql: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
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
        panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
    }

    pub(super) fn describe_query(
        &mut self,
        sql: &str,
        params: &super::Params,
    ) -> ConnectorResult<quaint::connector::DescribedQuery> {
        panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
    }
}

pub(super) fn generic_apply_migration_script(
    migration_name: &str,
    script: &str,
    conn: &Connection,
) -> ConnectorResult<()> {
    tracing::debug!(query_type = "raw_cmd", sql = script);
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn create_database(params: &super::Params) -> ConnectorResult<String> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn drop_database(params: &super::Params) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn ensure_connection_validity(params: &super::Params) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn introspect<'a>(
    instance: &'a mut super::SqliteFlavour,
    namespaces: Option<Namespaces>,
    _ctx: &schema_connector::IntrospectionContext,
) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn reset(params: &super::Params, connection: &mut Connection) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn version(instance: &mut super::SqliteFlavour) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}
