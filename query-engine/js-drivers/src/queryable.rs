use crate::driver::{self, Driver, JSResultSet, Query};
use async_trait::async_trait;
use napi::JsObject;
use quaint::{
    connector::IsolationLevel,
    prelude::{Query as QuaintQuery, Queryable as QuaintQueryable, ResultSet, TransactionCapable},
    visitor::{self, Visitor},
    Value,
};
use tracing::{info_span, Instrument};

#[derive(Clone)]
pub struct JsQueryable {
    pub(crate) driver: Driver,
}

impl JsQueryable {
    pub fn new(driver: Driver) -> Self {
        Self { driver }
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
        let (sql, params) = visitor::Mysql::build(q)?;
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
        let (sql, params) = visitor::Mysql::build(q)?;
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
        let version = self.driver.version().await.unwrap();
        Ok(version)
    }

    /// Returns false, if connection is considered to not be in a working state.
    fn is_healthy(&self) -> bool {
        // TODO: use self.driver.is_healthy()
        true
    }

    /// Sets the transaction isolation level to given value.
    /// Implementers have to make sure that the passed isolation level is valid for the underlying database.
    async fn set_tx_isolation_level(&self, _isolation_level: IsolationLevel) -> quaint::Result<()> {
        Ok(())
    }

    /// Signals if the isolation level SET needs to happen before or after the tx BEGIN.
    fn requires_isolation_first(&self) -> bool {
        false
    }
}

impl JsQueryable {
    async fn build_query(sql: &str, values: &[quaint::Value<'_>]) -> Query {
        let sql: String = sql.to_string();
        let args = values.iter().map(|v| v.clone().into()).collect();
        Query { sql, args }
    }

    async fn transform_result_set(result_set: JSResultSet) -> quaint::Result<ResultSet> {
        Ok(ResultSet::from(result_set))
    }

    async fn do_query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        let len = params.len();
        let serialization_span = info_span!("js:query:args", user_facing = true, "length" = %len);
        let query = Self::build_query(sql, params).instrument(serialization_span).await;

        // Todo: convert napi::Error to quaint::error::Error.
        let sql_span = info_span!("js:query:sql", user_facing = true, "db.statement" = %sql);
        let result_set = self.driver.query_raw(query).instrument(sql_span).await.unwrap();

        let len = result_set.len();
        let deserialization_span = info_span!("js:query:result", user_facing = true, "length" = %len);
        Self::transform_result_set(result_set)
            .instrument(deserialization_span)
            .await
    }

    async fn do_execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        let len = params.len();
        let serialization_span = info_span!("js:query:args", user_facing = true, "length" = %len);
        let query = Self::build_query(sql, params).instrument(serialization_span).await;

        // Todo: convert napi::Error to quaint::error::Error.
        let sql_span = info_span!("js:query:sql", user_facing = true, "db.statement" = %sql);
        let affected_rows = self.driver.execute_raw(query).instrument(sql_span).await.unwrap();

        Ok(affected_rows as u64)
    }
}

impl TransactionCapable for JsQueryable {}

impl From<JsObject> for JsQueryable {
    fn from(driver: JsObject) -> Self {
        let driver = driver::reify(driver).unwrap();
        Self { driver }
    }
}
