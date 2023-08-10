use async_trait::async_trait;
use napi::{bindgen_prelude::FromNapiValue, Env, JsObject};
use quaint::{
    connector::{IsolationLevel, Transaction as QuaintTransaction},
    prelude::{Query as QuaintQuery, Queryable, ResultSet},
    Value,
};

use crate::{
    error::into_quaint_error,
    proxy::{CommonProxy, TransactionProxy},
    queryable::JsBaseQueryable,
};

// Wrapper around JS transaction objects that implements Queryable
// and quaint::Transaction. Can be used in place of quaint transaction,
// but delegates most operations to JS
pub struct JsTransaction {
    tx_proxy: TransactionProxy,
    inner: JsBaseQueryable,
}

impl JsTransaction {
    pub fn new(inner: JsBaseQueryable, tx_proxy: TransactionProxy) -> Self {
        Self { inner, tx_proxy }
    }
}

#[async_trait]
impl QuaintTransaction for JsTransaction {
    /// Commit the changes to the database and consume the transaction.
    async fn commit(&self) -> quaint::Result<()> {
        self.tx_proxy.commit().await.map_err(into_quaint_error)
    }

    /// Rolls back the changes to the database.
    async fn rollback(&self) -> quaint::Result<()> {
        self.tx_proxy.rollback().await.map_err(into_quaint_error)
    }

    fn as_queryable(&self) -> &dyn Queryable {
        self
    }
}

#[async_trait]
impl Queryable for JsTransaction {
    /// Execute the given query.
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<ResultSet> {
        self.inner.query(q).await
    }

    /// Execute a query given as SQL, interpolating the given parameters.
    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    /// Execute a query given as SQL, interpolating the given parameters.
    ///
    /// On Postgres, query parameters types will be inferred from the values
    /// instead of letting Postgres infer them based on their usage in the SQL query.
    ///
    /// NOTE: This method will eventually be removed & merged into Queryable::query_raw().
    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw_typed(sql, params).await
    }

    /// Execute the given query, returning the number of affected rows.
    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
        self.inner.execute(q).await
    }

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    ///
    /// On Postgres, query parameters types will be inferred from the values
    /// instead of letting Postgres infer them based on their usage in the SQL query.
    ///
    /// NOTE: This method will eventually be removed & merged into Queryable::query_raw().
    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.inner.execute_raw_typed(sql, params).await
    }

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        self.inner.raw_cmd(cmd).await
    }

    /// Return the version of the underlying database, queried directly from the
    /// source. This corresponds to the `version()` function on PostgreSQL for
    /// example. The version string is returned directly without any form of
    /// parsing or normalization.
    async fn version(&self) -> quaint::Result<Option<String>> {
        self.inner.version().await
    }

    /// Returns false, if connection is considered to not be in a working state.
    fn is_healthy(&self) -> bool {
        self.inner.is_healthy()
    }

    /// Sets the transaction isolation level to given value.
    /// Implementers have to make sure that the passed isolation level is valid for the underlying database.
    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        self.inner.set_tx_isolation_level(isolation_level).await
    }

    /// Signals if the isolation level SET needs to happen before or after the tx BEGIN.
    fn requires_isolation_first(&self) -> bool {
        self.inner.requires_isolation_first()
    }
}

/// Implementing unsafe `from_napi_value` is only way I managed to get threadsafe
/// JsTransaction value in `DriverProxy`. Going through any intermediate safe napi.rs value,
/// like `JsObject` or `JsUnknown` wrapped inside `JsPromise` makes it impossible to extract the value
/// out of promise while keeping the future `Send`.
impl FromNapiValue for JsTransaction {
    unsafe fn from_napi_value(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        let object = JsObject::from_napi_value(env, napi_val)?;
        let env_safe = Env::from_raw(env);
        let common_proxy = CommonProxy::new(&object, &env_safe)?;
        let tx_proxy = TransactionProxy::new(&object, &env_safe)?;

        Ok(Self::new(JsBaseQueryable::new(common_proxy), tx_proxy))
    }
}
