use std::sync::Arc;

use async_trait::async_trait;

use crate::driver::Driver;

use quaint::{
    connector::IsolationLevel,
    prelude::{Query, Queryable as QuaintQueryable, TransactionCapable},
    visitor::{self, Visitor},
    Value,
};

#[derive(Clone)]
pub struct Queryable {
    /// A driver trait object wrapped in an [`Arc`].
    ///
    /// We need dynamic dispatch to ensure that the `js-drivers` crate and its users like
    /// `sql-query-connector` can be compiled separately from the concrete driver implementations,
    /// and that the former do not depend on the latter.
    ///
    /// When the Node.js driver implementation was a part of this crate, and `Queryable` used the
    /// concrete type and not a trait object, this made it not possible to compile the `js-drivers`
    /// as part of the Query Engine binary, since it would then fail with linker errors due to
    /// missing N-API symbols. Although a cargo feature was introduced for conditional compilation,
    /// due to shared dependencies it was still only possible to compile the Query Engine binary
    /// and the Node-API library separately but not together. While workarounds exist, like marking
    /// unknown symbols as weak symbols, they are platform (and linker) dependent and have other
    /// drawbacks.
    ///
    /// It should also be possible to parametrise `Queryable` with a generic type parameter for the
    /// [`Driver`] implementation and use static dispatch, if we want to eliminate the indirection
    /// here as a future optimisation. This will require changes downstream in the Query Engine
    /// code, as well as in how the `Driver` implementation is registered and stored.
    ///
    /// As for the type of the pointer, `Arc` provides the most straightforward way to allow the
    /// `Queryable` to be cloned. If we want to use `Box` in the future, that is also possible with
    /// a custom clone implementation (`dyn Driver` is not clonable by itself since it's a DST),
    /// however even a cloned `Driver` would currently share state on the JavaScript side.
    pub(crate) driver: Arc<dyn Driver>,
}

impl Queryable {
    pub fn new(driver: Arc<dyn Driver>) -> Self {
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

// Global JSQueryable instance serving as a proxy to the driver implemented by NodeJSFunctionContext
//
// It´s unlikely that we swap implementations, nor in production code, nor in test doubles, so relying
// on a global instance is fine.
static QUERYABLE: once_cell::sync::OnceCell<Queryable> = once_cell::sync::OnceCell::new();

pub fn install_driver(driver: Arc<dyn Driver>) {
    let queryable = Queryable::new(driver);
    QUERYABLE
        .set(queryable)
        .expect("Already initialized global instance of JSQueryable");
}

pub fn installed_driver() -> Option<&'static Queryable> {
    QUERYABLE.get()
}
