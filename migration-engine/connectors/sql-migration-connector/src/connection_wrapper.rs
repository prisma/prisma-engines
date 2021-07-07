use crate::error::quaint_error_to_connector_error;
use migration_connector::ConnectorError;
use quaint::{
    connector::{Mysql, MysqlUrl, PostgreSql, PostgresUrl},
    error::{Error as QuaintError, ErrorKind as QuaintKind},
    prelude::{ConnectionInfo, Query, Queryable, ResultSet},
    single::Quaint,
};
use std::sync::Arc;

/// An internal helper for the SQL connector. It wraps a `Quaint` struct and
/// exposes a similar API, with additional error handling to return
/// `ConnectorResult`s.
#[derive(Clone, Debug)]
pub(crate) struct Connection(ConnectionInner);

#[derive(Clone, Debug)]
enum ConnectionInner {
    Postgres(Arc<(quaint::connector::PostgreSql, PostgresUrl)>),
    Mysql(Arc<(quaint::connector::Mysql, MysqlUrl)>),
    Generic(Quaint),
}

#[derive(Debug)]
pub(crate) struct ConnectionError {
    quaint_error: QuaintError,
    connection_info: ConnectionInfo,
}

type ConnectionResult<T> = Result<T, ConnectionError>;

impl ConnectionError {
    pub(crate) fn kind(&self) -> &QuaintKind {
        self.quaint_error.kind()
    }

    pub(crate) fn original_code(&self) -> Option<&str> {
        self.quaint_error.original_code()
    }

    pub(crate) fn original_message(&self) -> Option<&str> {
        self.quaint_error.original_message()
    }
}

impl From<ConnectionError> for ConnectorError {
    fn from(err: ConnectionError) -> Self {
        quaint_error_to_connector_error(err.quaint_error, &err.connection_info)
    }
}

impl Connection {
    pub(crate) fn new_generic(quaint: Quaint) -> Self {
        Connection(ConnectionInner::Generic(quaint))
    }

    pub(crate) fn new_postgres(conn: PostgreSql, url: PostgresUrl) -> Self {
        Connection(ConnectionInner::Postgres(Arc::new((conn, url))))
    }

    pub(crate) fn new_mysql(conn: Mysql, url: MysqlUrl) -> Self {
        Connection(ConnectionInner::Mysql(Arc::new((conn, url))))
    }

    pub(crate) fn connection_info(&self) -> ConnectionInfo {
        match &self.0 {
            ConnectionInner::Postgres(pg) => ConnectionInfo::Postgres(pg.1.clone()),
            ConnectionInner::Mysql(my) => ConnectionInfo::Mysql(my.1.clone()),
            ConnectionInner::Generic(q) => q.connection_info().clone(),
        }
    }

    pub(crate) async fn execute(&self, query: impl Into<Query<'_>>) -> ConnectionResult<u64> {
        self.queryable()
            .execute(query.into())
            .await
            .map_err(|quaint_error| ConnectionError {
                quaint_error,
                connection_info: self.connection_info(),
            })
    }

    pub(crate) fn queryable(&self) -> &dyn Queryable {
        match &self.0 {
            ConnectionInner::Postgres(pg) => &pg.0,
            ConnectionInner::Mysql(my) => &my.0,
            ConnectionInner::Generic(q) => q,
        }
    }

    pub(crate) async fn query(&self, query: impl Into<Query<'_>>) -> ConnectionResult<ResultSet> {
        self.queryable()
            .query(query.into())
            .await
            .map_err(|quaint_error| ConnectionError {
                quaint_error,
                connection_info: self.connection_info(),
            })
    }

    pub(crate) async fn query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> ConnectionResult<ResultSet> {
        self.queryable()
            .query_raw(sql, params)
            .await
            .map_err(|quaint_error| ConnectionError {
                quaint_error,
                connection_info: self.connection_info(),
            })
    }

    pub(crate) async fn raw_cmd(&self, sql: &str) -> ConnectionResult<()> {
        self.queryable()
            .raw_cmd(sql)
            .await
            .map_err(|quaint_error| ConnectionError {
                quaint_error,
                connection_info: self.connection_info(),
            })
    }

    pub(crate) fn schema_name(&self) -> &str {
        match &self.0 {
            ConnectionInner::Postgres(pg) => pg.1.schema(),
            ConnectionInner::Mysql(my) => my.1.dbname(),
            ConnectionInner::Generic(quaint) => quaint.connection_info().schema_name(),
        }
    }

    pub(crate) async fn version(&self) -> ConnectionResult<Option<String>> {
        self.queryable()
            .version()
            .await
            .map_err(|quaint_error| ConnectionError {
                quaint_error,
                connection_info: self.connection_info(),
            })
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub(crate) fn table_name<'a>(&'a self, name: &'a str) -> quaint::ast::Table<'a> {
        if self.connection_info().sql_family().is_sqlite() {
            name.into()
        } else {
            (self.schema_name(), name).into()
        }
    }

    pub(crate) fn unwrap_postgres(&self) -> &(PostgreSql, PostgresUrl) {
        match &self.0 {
            ConnectionInner::Postgres(inner) => inner,
            other => panic!("{:?} in Connection::unwrap_postgres()", other),
        }
    }

    pub(crate) fn unwrap_mysql(&self) -> &(Mysql, MysqlUrl) {
        match &self.0 {
            ConnectionInner::Mysql(inner) => &**inner,
            other => panic!("{:?} in Connection::unwrap_mysql()", other),
        }
    }
}
