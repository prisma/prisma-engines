use async_trait::async_trait;
use futures::lock::Mutex;
use metrics::decrement_gauge;
use napi::{bindgen_prelude::FromNapiValue, JsObject};
use quaint::{
    connector::{IsolationLevel, Transaction as QuaintTransaction},
    prelude::{Query as QuaintQuery, Queryable, ResultSet},
    Value,
};
use std::sync::Arc;

use crate::{
    proxy::{CommonProxy, TransactionOptions, TransactionProxy},
    queryable::JsBaseQueryable,
};

// Wrapper around JS transaction objects that implements Queryable
// and quaint::Transaction. Can be used in place of quaint transaction,
// but delegates most operations to JS
pub(crate) struct JsTransaction {
    tx_proxy: TransactionProxy,
    inner: JsBaseQueryable,
    pub depth: Arc<Mutex<i32>>,
    pub commit_stmt: String,
    pub rollback_stmt: String,
}

impl JsTransaction {
    pub(crate) fn new(inner: JsBaseQueryable, tx_proxy: TransactionProxy) -> Self {
        Self {
            inner,
            tx_proxy,
            commit_stmt: "COMMIT".to_string(),
            rollback_stmt: "ROLLBACK".to_string(),
            depth: Arc::new(futures::lock::Mutex::new(0)),
        }
    }

    pub fn options(&self) -> &TransactionOptions {
        self.tx_proxy.options()
    }

    pub async fn raw_phantom_cmd(&self, cmd: &str) -> quaint::Result<()> {
        let params = &[];
        quaint::connector::metrics::query("js.raw_phantom_cmd", cmd, params, move || async move { Ok(()) }).await
    }
}

#[async_trait]
impl QuaintTransaction for JsTransaction {
    async fn begin(&mut self) -> quaint::Result<()> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let mut depth_guard = self.depth.lock().await;
        let commit_stmt = "BEGIN";

        if self.options().use_phantom_query {
            let commit_stmt = JsBaseQueryable::phantom_query_message(commit_stmt);
            self.raw_phantom_cmd(commit_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(commit_stmt).await?;
        }

        // Modify the depth value through the MutexGuard
        *depth_guard += 1;

        self.tx_proxy.begin().await
    }
    async fn commit(&mut self) -> quaint::Result<i32> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let mut depth_guard = self.depth.lock().await;
        let commit_stmt = &self.commit_stmt;

        if self.options().use_phantom_query {
            let commit_stmt = JsBaseQueryable::phantom_query_message(commit_stmt);
            self.raw_phantom_cmd(commit_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(commit_stmt).await?;
        }

        // Modify the depth value through the MutexGuard
        *depth_guard -= 1;

        let _ = self.tx_proxy.commit().await;

        Ok(*depth_guard)
    }

    async fn rollback(&mut self) -> quaint::Result<i32> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let mut depth_guard = self.depth.lock().await;
        let rollback_stmt = &self.rollback_stmt;

        if self.options().use_phantom_query {
            let rollback_stmt = JsBaseQueryable::phantom_query_message(rollback_stmt);
            self.raw_phantom_cmd(rollback_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(rollback_stmt).await?;
        }

        // Modify the depth value through the MutexGuard
        *depth_guard -= 1;

        let _ = self.tx_proxy.rollback().await;

        Ok(*depth_guard)
    }

    fn as_queryable(&self) -> &dyn Queryable {
        self
    }
}

#[async_trait]
impl Queryable for JsTransaction {
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<ResultSet> {
        self.inner.query(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw_typed(sql, params).await
    }

    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
        self.inner.execute(q).await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> quaint::Result<u64> {
        self.inner.execute_raw_typed(sql, params).await
    }

    async fn raw_cmd(&self, cmd: &str) -> quaint::Result<()> {
        self.inner.raw_cmd(cmd).await
    }

    async fn version(&self) -> quaint::Result<Option<String>> {
        self.inner.version().await
    }

    fn is_healthy(&self) -> bool {
        self.inner.is_healthy()
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> quaint::Result<()> {
        self.inner.set_tx_isolation_level(isolation_level).await
    }

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
        let common_proxy = CommonProxy::new(&object)?;
        let tx_proxy = TransactionProxy::new(&object)?;

        Ok(Self::new(JsBaseQueryable::new(common_proxy), tx_proxy))
    }
}
