use quaint::{
    ast::*,
    connector::{ResultSet},
    error::Error as QueryError,
};

/// A generic synchronous SQL connection interface.
pub trait SyncSqlConnection {
    /// See
    /// https://prisma.github.io/quaint/quaint/connector/trait.Queryable.html#tymethod.execute
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    fn execute(&self, q: Query<'_>) -> Result<Option<Id>, QueryError>;

    /// See
    /// https://prisma.github.io/quaint/quaint/connector/trait.Queryable.html#tymethod.query
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    fn query(&self, q: Query<'_>) -> Result<ResultSet, QueryError>;

    /// See
    /// https://prisma.github.io/quaint/quaint/connector/trait.Queryable.html#tymethod.query_raw
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    fn query_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<ResultSet, QueryError>;

    /// See
    /// https://prisma.github.io/quaint/quaint/connector/trait.Queryable.html#tymethod.execute_raw
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    fn execute_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<u64, QueryError>;
}

/// A generic asynchronous SQL connection interface.
#[async_trait::async_trait]
pub trait SqlConnection: Send + Sync + 'static {
    /// See
    /// https://prisma.github.io/quaint/quaint/connector/trait.Queryable.html#tymethod.execute
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    async fn execute<'a>(&self, q: Query<'a>) -> Result<Option<Id>, QueryError>;

    /// See
    /// https://prisma.github.io/quaint/quaint/connector/trait.Queryable.html#tymethod.query
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    async fn query<'a>(&self, q: Query<'a>) -> Result<ResultSet, QueryError>;

    /// See
    /// https://prisma.github.io/quaint/quaint/connector/trait.Queryable.html#tymethod.query_raw
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    async fn query_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<ResultSet, QueryError>;

    /// See
    /// https://prisma.github.io/quaint/quaint/connector/trait.Queryable.html#tymethod.execute_raw
    ///
    /// The `db` param is only used on SQLite to give a name to the attached database.
    async fn execute_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QueryError>;
}
