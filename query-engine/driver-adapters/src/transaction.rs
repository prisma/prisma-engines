use async_trait::async_trait;
use futures::lock::Mutex;
use metrics::decrement_gauge;
use quaint::{
    connector::{DescribedQuery, IsolationLevel, Transaction as QuaintTransaction},
    prelude::{Query as QuaintQuery, Queryable, ResultSet},
    Value,
};
use std::sync::Arc;

use crate::proxy::{TransactionOptions, TransactionProxy};
use crate::{proxy::CommonProxy, queryable::JsBaseQueryable, send_future::UnsafeFuture};
use crate::{JsObject, JsResult};

// Wrapper around JS transaction objects that implements Queryable
// and quaint::Transaction. Can be used in place of quaint transaction,
// but delegates most operations to JS
pub(crate) struct JsTransaction {
    tx_proxy: TransactionProxy,
    inner: JsBaseQueryable,
    pub depth: Arc<Mutex<i32>>,
}

impl JsTransaction {
    pub(crate) fn new(inner: JsBaseQueryable, tx_proxy: TransactionProxy) -> Self {
        Self {
            inner,
            tx_proxy,
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
        // Modify the depth value through the MutexGuard
        *depth_guard += 1;

        let begin_stmt = self.begin_statement(*depth_guard).await;

        if self.options().use_phantom_query {
            let commit_stmt = JsBaseQueryable::phantom_query_message(&begin_stmt);
            self.raw_phantom_cmd(commit_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(&begin_stmt).await?;
        }

        println!("JsTransaction begin: incrementing depth_guard to: {}", *depth_guard);

        UnsafeFuture(self.tx_proxy.begin()).await
    }
    async fn commit(&mut self) -> quaint::Result<i32> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let mut depth_guard = self.depth.lock().await;
        let commit_stmt = self.commit_statement(*depth_guard).await;

        if self.options().use_phantom_query {
            let commit_stmt = JsBaseQueryable::phantom_query_message(&commit_stmt);
            self.raw_phantom_cmd(commit_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(&commit_stmt).await?;
        }

        // Modify the depth value through the MutexGuard
        *depth_guard -= 1;

        let _ = UnsafeFuture(self.tx_proxy.commit()).await;

        Ok(*depth_guard)
    }

    async fn rollback(&mut self) -> quaint::Result<i32> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let mut depth_guard = self.depth.lock().await;
        let rollback_stmt = self.rollback_statement(*depth_guard).await;

        if self.options().use_phantom_query {
            let rollback_stmt = JsBaseQueryable::phantom_query_message(&rollback_stmt);
            self.raw_phantom_cmd(rollback_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(&rollback_stmt).await?;
        }

        // Modify the depth value through the MutexGuard
        *depth_guard -= 1;

        let _ = UnsafeFuture(self.tx_proxy.rollback()).await;

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

    async fn describe_query(&self, sql: &str) -> quaint::Result<DescribedQuery> {
        self.inner.describe_query(sql).await
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

    async fn begin_statement(&self, depth: i32) -> String {
        self.inner.begin_statement(depth).await
    }

    async fn commit_statement(&self, depth: i32) -> String {
        self.inner.commit_statement(depth).await
    }

    async fn rollback_statement(&self, depth: i32) -> String {
        self.inner.rollback_statement(depth).await
    }
}

#[cfg(target_arch = "wasm32")]
impl super::wasm::FromJsValue for JsTransaction {
    fn from_js_value(value: wasm_bindgen::prelude::JsValue) -> JsResult<Self> {
        use wasm_bindgen::JsCast;

        let object = value.dyn_into::<JsObject>()?;
        let common_proxy = CommonProxy::new(&object)?;
        let base = JsBaseQueryable::new(common_proxy);
        let tx_proxy = TransactionProxy::new(&object)?;

        Ok(Self::new(base, tx_proxy))
    }
}

/// Implementing unsafe `from_napi_value` allows retrieving a threadsafe `JsTransaction` in `DriverProxy`
/// while keeping derived futures `Send`.
#[cfg(not(target_arch = "wasm32"))]
impl ::napi::bindgen_prelude::FromNapiValue for JsTransaction {
    unsafe fn from_napi_value(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> JsResult<Self> {
        let object = JsObject::from_napi_value(env, napi_val)?;
        let common_proxy = CommonProxy::new(&object)?;
        let tx_proxy = TransactionProxy::new(&object)?;

        Ok(Self::new(JsBaseQueryable::new(common_proxy), tx_proxy))
    }
}
