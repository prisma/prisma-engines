use std::{
    borrow::Cow,
    sync::atomic::{AtomicI32, Ordering},
};

use async_trait::async_trait;
use quaint::{
    Value,
    connector::{DescribedQuery, IsolationLevel, Transaction as QuaintTransaction},
    prelude::{Query as QuaintQuery, Queryable, ResultSet},
};

use crate::proxy::{TransactionOptions, TransactionProxy};
use crate::{proxy::CommonProxy, queryable::JsBaseQueryable, send_future::UnsafeFuture};
use crate::{JsObject, JsResult};

// Wrapper around JS transaction objects that implements Queryable and quaint::Transaction.
// It delegates the underlying transaction lifecycle to the JS driver adapter.
pub(crate) struct JsTransaction {
    tx_proxy: TransactionProxy,
    inner: JsBaseQueryable,
    pub depth: AtomicI32,
}

impl JsTransaction {
    pub(crate) fn new(inner: JsBaseQueryable, tx_proxy: TransactionProxy) -> Self {
        Self {
            inner,
            tx_proxy,
            depth: AtomicI32::new(0),
        }
    }

    pub fn options(&self) -> &TransactionOptions {
        self.tx_proxy.options()
    }

    pub async fn raw_phantom_cmd(&self, cmd: &str) -> quaint::Result<()> {
        let params = &[];
        quaint::connector::trace::query(self.inner.db_system_name, cmd, params, move || async move { Ok(()) }).await
    }

    pub fn increment_depth(&self) {
        self.depth.fetch_add(1, Ordering::Relaxed);
    }
}

#[async_trait]
impl QuaintTransaction for JsTransaction {
    fn depth(&self) -> i32 {
        self.depth.load(Ordering::Relaxed)
    }

    async fn begin(&self) -> quaint::Result<()> {
        // increment of this gauge is done in DriverProxy::startTransaction
        gauge!("prisma_client_queries_active").decrement(1.0);

        self.depth.fetch_add(1, Ordering::Relaxed);

        let begin_stmt = self.begin_statement();

        if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(begin_stmt);
            self.raw_phantom_cmd(phantom.as_str()).await?;
        } else {
            self.inner.raw_cmd(begin_stmt).await?;
        }

        UnsafeFuture(self.tx_proxy.begin()).await
    }

    async fn commit(&self) -> quaint::Result<()> {
        // increment of this gauge is done in DriverProxy::startTransaction
        gauge!("prisma_client_queries_active").decrement(1.0);

        // Reset the depth to 0 on commit.
        self.depth.store(0, Ordering::Relaxed);

        let commit_stmt = "COMMIT";

        if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(commit_stmt);
            self.raw_phantom_cmd(phantom.as_str()).await?;
        } else {
            self.inner.raw_cmd(commit_stmt).await?;
        }

        let _ = UnsafeFuture(self.tx_proxy.commit()).await;

        Ok(())
    }

    async fn rollback(&self) -> quaint::Result<()> {
        // increment of this gauge is done in DriverProxy::startTransaction
        gauge!("prisma_client_queries_active").decrement(1.0);

        self.depth.fetch_sub(1, Ordering::Relaxed);

        let rollback_stmt = "ROLLBACK";

        if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(rollback_stmt);
            self.raw_phantom_cmd(phantom.as_str()).await?;
        } else {
            self.inner.raw_cmd(rollback_stmt).await?;
        }

        let _ = UnsafeFuture(self.tx_proxy.rollback()).await;

        Ok(())
    }

    async fn create_savepoint(&self) -> quaint::Result<()> {
        let new_depth = self.depth.fetch_add(1, Ordering::Relaxed) + 1;

        let create_savepoint_statement = self.create_savepoint_statement(new_depth as u32);
        if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(&create_savepoint_statement);
            self.raw_phantom_cmd(phantom.as_str()).await?;
        } else {
            self.inner.raw_cmd(&create_savepoint_statement).await?;
        }

        Ok(())
    }

    async fn release_savepoint(&self) -> quaint::Result<()> {
        let depth_val = self.depth.fetch_sub(1, Ordering::Relaxed);

        let release_savepoint_statement = self.release_savepoint_statement(depth_val as u32);
        if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(&release_savepoint_statement);
            self.raw_phantom_cmd(phantom.as_str()).await?;
        } else {
            self.inner.raw_cmd(&release_savepoint_statement).await?;
        }

        Ok(())
    }

    async fn rollback_to_savepoint(&self) -> quaint::Result<()> {
        let depth_val = self.depth.fetch_sub(1, Ordering::Relaxed);

        let rollback_to_savepoint_statement = self.rollback_to_savepoint_statement(depth_val as u32);
        if self.options().use_phantom_query {
            let phantom = JsBaseQueryable::phantom_query_message(&rollback_to_savepoint_statement);
            self.raw_phantom_cmd(phantom.as_str()).await?;
        } else {
            self.inner.raw_cmd(&rollback_to_savepoint_statement).await?;
        }

        Ok(())
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

    fn begin_statement(&self) -> &'static str {
        self.inner.begin_statement()
    }

    fn create_savepoint_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.create_savepoint_statement(depth)
    }

    fn release_savepoint_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.release_savepoint_statement(depth)
    }

    fn rollback_to_savepoint_statement(&self, depth: u32) -> Cow<'static, str> {
        self.inner.rollback_to_savepoint_statement(depth)
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
