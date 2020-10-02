use crate::error::quaint_error_to_connector_error;
use migration_connector::ConnectorResult;
use quaint::{
    prelude::{ConnectionInfo, Query, Queryable, ResultSet},
    single::Quaint,
};

/// An internal helper for the SQL connector. It wraps a `Quaint` struct and
/// exposes a similar API, with additional error handling to return
/// `ConnectorResult`s.
#[derive(Clone, Debug)]
pub(crate) struct Connection(Quaint);

impl Connection {
    pub(crate) fn new(quaint: Quaint) -> Self {
        Connection(quaint)
    }

    pub(crate) fn connection_info(&self) -> &ConnectionInfo {
        self.0.connection_info()
    }

    pub(crate) async fn execute(&self, query: impl Into<Query<'_>>) -> ConnectorResult<u64> {
        self.0
            .execute(query.into())
            .await
            .map_err(|err| quaint_error_to_connector_error(err, self.connection_info()))
    }

    pub(crate) fn quaint(&self) -> &Quaint {
        &self.0
    }

    pub(crate) async fn query(&self, query: impl Into<Query<'_>>) -> ConnectorResult<ResultSet> {
        self.0
            .query(query.into())
            .await
            .map_err(|err| quaint_error_to_connector_error(err, self.connection_info()))
    }

    pub(crate) async fn query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> ConnectorResult<ResultSet> {
        self.0
            .query_raw(sql, params)
            .await
            .map_err(|err| quaint_error_to_connector_error(err, self.connection_info()))
    }

    pub(crate) async fn raw_cmd(&self, sql: &str) -> ConnectorResult<()> {
        self.0
            .raw_cmd(sql)
            .await
            .map_err(|err| quaint_error_to_connector_error(err, self.connection_info()))
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub(crate) fn table_name<'a>(&'a self, name: &'a str) -> quaint::ast::Table<'a> {
        if self.connection_info().sql_family().is_sqlite() {
            name.into()
        } else {
            (self.connection_info().schema_name(), name).into()
        }
    }
}
