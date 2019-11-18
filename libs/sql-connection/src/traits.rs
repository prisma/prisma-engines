use quaint::{ast::*, connector::ResultSet, error::Error as QueryError};

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
