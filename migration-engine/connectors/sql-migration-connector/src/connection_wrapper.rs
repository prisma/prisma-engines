use migration_connector::ConnectorResult;
use quaint::{
    prelude::{ConnectionInfo, Query, Queryable, ResultSet},
    single::Quaint,
};

use crate::SqlError;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Connection<'a>(&'a Quaint);

impl<'a> Connection<'a> {
    pub(crate) fn new(quaint: &'a Quaint) -> Self {
        Connection(quaint)
    }

    pub(crate) fn connection_info(&self) -> &ConnectionInfo {
        self.0.connection_info()
    }

    pub(crate) async fn execute(&self, query: impl Into<Query<'_>>) -> ConnectorResult<u64> {
        self.0
            .execute(query.into())
            .await
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(self.0.connection_info()))
    }

    pub(crate) async fn execute_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> ConnectorResult<u64> {
        self.0
            .execute_raw(sql, params)
            .await
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(self.0.connection_info()))
    }

    pub(crate) fn quaint(&self) -> &Quaint {
        self.0
    }

    pub(crate) async fn query(&self, query: impl Into<Query<'_>>) -> ConnectorResult<ResultSet> {
        self.0
            .query(query.into())
            .await
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(self.0.connection_info()))
    }

    pub(crate) async fn query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> ConnectorResult<ResultSet> {
        self.0
            .query_raw(sql, params)
            .await
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(self.0.connection_info()))
    }

    pub(crate) async fn raw_cmd(&self, sql: &str) -> ConnectorResult<()> {
        self.0
            .raw_cmd(sql)
            .await
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(self.0.connection_info()))
    }
}
