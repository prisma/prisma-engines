use crate::driver::{self, Driver};
use async_trait::async_trait;
use quaint::{
    connector::IsolationLevel,
    prelude::{Query, Queryable as QuaintQueryable, TransactionCapable},
    visitor::{self, Visitor},
    Value,
};

use napi::JsObject;

#[derive(Clone)]
pub struct Queryable {
    pub(crate) driver: Driver,
}

impl Queryable {
    pub fn new(driver: Driver) -> Self {
        Self { driver }
    }
}

impl std::fmt::Display for Queryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSQueryable(driver)")
    }
}

impl std::fmt::Debug for Queryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSQueryable(driver)")
    }
}

#[async_trait]
impl QuaintQueryable for Queryable {
    /// Execute the given query.
    async fn query(&self, q: Query<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        let (sql, params) = visitor::Mysql::build(q)?;
        println!("JSQueryable::query()");
        self.query_raw(&sql, &params).await
    }

    /// Execute a query given as SQL, interpolating the given parameters.
    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        println!("JSQueryable::query_raw({}, {:?})", &sql, params);

        // Note: we ignore the parameters for now.
        // Todo: convert napi::Error to quaint::error::Error.
        let result_set = self.driver.query_raw(sql.to_string()).await.unwrap();

        Ok(quaint::prelude::ResultSet::new(
            result_set.columns,
            result_set
                .rows
                .into_iter()
                .map(|row| row.into_iter().map(quaint::Value::text).collect())
                .collect(),
        ))
    }

    /// Execute a query given as SQL, interpolating the given parameters.
    ///
    /// On Postgres, query parameters types will be inferred from the values
    /// instead of letting Postgres infer them based on their usage in the SQL query.
    ///
    /// NOTE: This method will eventually be removed & merged into Queryable::query_raw().
    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        println!("JSQueryable::query_raw()");
        self.query_raw(sql, params).await
    }

    /// Execute the given query, returning the number of affected rows.
    async fn execute(&self, q: Query<'_>) -> quaint::Result<u64> {
        println!("JSQueryable::execute()");
        let (sql, params) = visitor::Mysql::build(q)?;
        self.execute_raw(&sql, &params).await
    }

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        println!("JSQueryable::execute_raw({}, {:?})", &sql, &params);

        // Note: we ignore the parameters for now.
        // Todo: convert napi::Error to quaint::error::Error.
        let affected_rows = self.driver.execute_raw(sql.to_string()).await.unwrap();
        Ok(affected_rows as u64)
    }

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    ///
    /// On Postgres, query parameters types will be inferred from the values
    /// instead of letting Postgres infer them based on their usage in the SQL query.
    ///
    /// NOTE: This method will eventually be removed & merged into Queryable::query_raw().
    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        println!("JSQueryable::execute_raw_typed({}, {:?})", &sql, &params);
        self.execute_raw(sql, params).await
    }

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        println!("JSQueryable::raw_cmdx({})", &cmd);
        self.execute_raw(cmd, &[]).await?;

        Ok(())
    }

    /// Return the version of the underlying database, queried directly from the
    /// source. This corresponds to the `version()` function on PostgreSQL for
    /// example. The version string is returned directly without any form of
    /// parsing or normalization.
    async fn version(&self) -> quaint::Result<Option<String>> {
        println!("JSQueryable::version()");
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

impl TransactionCapable for Queryable {}

impl From<JsObject> for Queryable {
    fn from(driver: JsObject) -> Self {
        let driver = driver::reify(driver).unwrap();
        Self { driver }
    }
}
