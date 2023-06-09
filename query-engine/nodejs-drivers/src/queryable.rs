use async_trait::async_trait;

use crate::driver::{Driver, Query};

use quaint::{
    connector::IsolationLevel,
    prelude::{Query as QuaintQuery, Queryable as QuaintQueryable, TransactionCapable},
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
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        let (sql, params) = visitor::Mysql::build(q)?;
        println!("JSQueryable::query()");
        self.query_raw(&sql, &params).await
    }

    /// Execute a query given as SQL, interpolating the given parameters.
    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<quaint::prelude::ResultSet> {
        println!("JSQueryable::query_raw({}, {:?})", &sql, params);

        // Note: we ignore the parameters for now.
        // Todo: convert napi::Error to quaint::error::Error.
        let result_set = self.driver.query_raw((sql, params).into()).await.unwrap();

        Ok(result_set.into())
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
    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
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
        let affected_rows = self.driver.execute_raw((sql, params).into()).await.unwrap();
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

// Global JSQueryable instance serving as a proxy to the driver implemented by NodeJSFunctionContext
//
// It´s unlikely that we swap implementations, nor in production code, nor in test doubles, so relying
// on a global instance is fine.
static QUERYABLE: once_cell::sync::OnceCell<Queryable> = once_cell::sync::OnceCell::new();

pub fn install_driver(js_driver: JsObject) {
    let driver = crate::driver::reify(js_driver).unwrap();
    let nodejs_queryable = Queryable::new(driver);
    QUERYABLE
        .set(nodejs_queryable)
        .expect("Already initialized global instance of JSQueryable");
}

pub fn installed_driver() -> Option<&'static Queryable> {
    QUERYABLE.get()
}

impl From<(&str, &[quaint::Value<'_>])> for Query {
    fn from(value: (&str, &[quaint::Value])) -> Self {
        let sql = value.0.to_string();
        let args = value.1.iter().map(|v| v.clone().into()).collect();
        Query { sql, args }
    }
}
