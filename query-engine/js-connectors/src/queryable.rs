use crate::{
    error::into_quaint_error,
    proxy::{self, Proxy, Query},
};
use async_trait::async_trait;
use napi::{Env, JsObject};
use psl::datamodel_connector::Flavour;
use quaint::{
    connector::IsolationLevel,
    error::{Error, ErrorKind},
    prelude::{Query as QuaintQuery, Queryable as QuaintQueryable, ResultSet, TransactionCapable},
    visitor::{self, Visitor},
    Value,
};
use tracing::{info_span, Instrument};

/// A JsQueryable adapts a Proxy to implement quaint's Queryable interface. It has the
/// responsibility of transforming inputs and outputs of `query` and `execute` methods from quaint
/// types to types that can be translated into javascript and viceversa. This is to let the rest of
/// the query engine work as if it was using quaint itself. The aforementioned transformations are:
///
/// Transforming a `quaint::ast::Query` into SQL by visiting it for the specific flavour of SQL
/// expected by the client connector. (eg. using the mysql visitor for the Planetscale client
/// connector)
///
/// Transforming a `JSResultSet` (what client connectors implemented in javascript provide)
/// into a `quaint::connector::result_set::ResultSet`. A quaint `ResultSet` is basically a vector
/// of `quaint::Value` but said type is a tagged enum, with non-unit variants that cannot be converted to javascript as is.
///
pub struct JsQueryable {
    pub(crate) proxy: Proxy,
    pub flavour: Flavour,
}

impl JsQueryable {
    pub fn new(proxy: Proxy, flavour: Flavour) -> Self {
        Self { proxy, flavour }
    }

    /// visit a query according to the flavour of the JS connector
    pub fn visit_query<'a>(&self, q: QuaintQuery<'a>) -> quaint::Result<(String, Vec<Value<'a>>)> {
        match self.flavour {
            Flavour::Mysql => visitor::Mysql::build(q),
            Flavour::Postgres => visitor::Postgres::build(q),
            _ => unimplemented!("Unsupported flavour for JS connector {:?}", self.flavour),
        }
    }
}

impl std::fmt::Display for JsQueryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSQueryable(driver)")
    }
}

impl std::fmt::Debug for JsQueryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSQueryable(driver)")
    }
}

#[async_trait]
impl QuaintQueryable for JsQueryable {
    /// Execute the given query.
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<ResultSet> {
        let (sql, params) = self.visit_query(q)?;
        self.query_raw(&sql, &params).await
    }

    /// Execute a query given as SQL, interpolating the given parameters.
    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        let span = info_span!("js:query", user_facing = true);
        self.do_query_raw(sql, params).instrument(span).await
    }

    /// Execute a query given as SQL, interpolating the given parameters.
    ///
    /// On Postgres, query parameters types will be inferred from the values
    /// instead of letting Postgres infer them based on their usage in the SQL query.
    ///
    /// NOTE: This method will eventually be removed & merged into Queryable::query_raw().
    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        self.query_raw(sql, params).await
    }

    /// Execute the given query, returning the number of affected rows.
    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
        let (sql, params) = self.visit_query(q)?;
        self.execute_raw(&sql, &params).await
    }

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        let span = info_span!("js:query", user_facing = true);
        self.do_execute_raw(sql, params).instrument(span).await
    }

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    ///
    /// On Postgres, query parameters types will be inferred from the values
    /// instead of letting Postgres infer them based on their usage in the SQL query.
    ///
    /// NOTE: This method will eventually be removed & merged into Queryable::query_raw().
    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.execute_raw(sql, params).await
    }

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        self.execute_raw(cmd, &[]).await?;
        Ok(())
    }

    /// Return the version of the underlying database, queried directly from the
    /// source. This corresponds to the `version()` function on PostgreSQL for
    /// example. The version string is returned directly without any form of
    /// parsing or normalization.
    async fn version(&self) -> quaint::Result<Option<String>> {
        // Todo: convert napi::Error to quaint::error::Error.
        let version = self.proxy.version().await.unwrap();
        Ok(version)
    }

    /// Returns false, if connection is considered to not be in a working state.
    fn is_healthy(&self) -> bool {
        self.proxy.is_healthy().unwrap_or(false)
    }

    /// Sets the transaction isolation level to given value.
    /// Implementers have to make sure that the passed isolation level is valid for the underlying database.
    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        if matches!(isolation_level, IsolationLevel::Snapshot) {
            return Err(Error::builder(ErrorKind::invalid_isolation_level(&isolation_level)).build());
        }

        self.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation_level}"))
            .await?;

        Ok(())
    }

    /// Signals if the isolation level SET needs to happen before or after the tx BEGIN.
    fn requires_isolation_first(&self) -> bool {
        match self.flavour {
            Flavour::Mysql => true,
            Flavour::Postgres | Flavour::Sqlite => false,
            _ => unreachable!(),
        }
    }
}

impl JsQueryable {
    async fn build_query(sql: &str, values: &[quaint::Value<'_>]) -> Query {
        let sql: String = sql.to_string();
        let args = values.iter().map(|v| v.clone().into()).collect();
        Query { sql, args }
    }

    async fn do_query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        let len = params.len();
        let serialization_span = info_span!("js:query:args", user_facing = true, "length" = %len);
        let query = Self::build_query(sql, params).instrument(serialization_span).await;

        let sql_span = info_span!("js:query:sql", user_facing = true, "db.statement" = %sql);
        let result_set = self
            .proxy
            .query_raw(query)
            .instrument(sql_span)
            .await
            .map_err(into_quaint_error)?;

        let len = result_set.len();
        let _deserialization_span = info_span!("js:query:result", user_facing = true, "length" = %len).entered();
        Ok(ResultSet::from(result_set))
    }

    async fn do_execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        let len = params.len();
        let serialization_span = info_span!("js:query:args", user_facing = true, "length" = %len);
        let query = Self::build_query(sql, params).instrument(serialization_span).await;

        // Todo: convert napi::Error to quaint::error::Error.
        let sql_span = info_span!("js:query:sql", user_facing = true, "db.statement" = %sql);
        let affected_rows = self.proxy.execute_raw(query).instrument(sql_span).await.unwrap();

        Ok(affected_rows as u64)
    }
}

impl TransactionCapable for JsQueryable {}

pub fn from_napi(napi_env: &Env, driver: JsObject) -> JsQueryable {
    let driver = proxy::reify(napi_env, driver).unwrap();
    let flavour = driver.flavour.parse().unwrap();
    JsQueryable::new(driver, flavour)
}
