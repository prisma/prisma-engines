use std::future::Future;

use async_trait::async_trait;
use metrics::decrement_gauge;
use quaint::{
    connector::{DescribedQuery, IsolationLevel, Transaction as QuaintTransaction},
    prelude::{Query as QuaintQuery, Queryable, ResultSet},
    Value,
};

use crate::proxy::{TransactionContextProxy, TransactionOptions, TransactionProxy};
use crate::{proxy::CommonProxy, queryable::JsBaseQueryable, send_future::UnsafeFuture};
use crate::{JsObject, JsResult};

pub(crate) struct JsTransactionContext {
    tx_ctx_proxy: TransactionContextProxy,
    inner: JsBaseQueryable,
}

// Wrapper around JS transaction context objects that implements Queryable. Can be used in place of quaint transaction,
// context, but delegates most operations to JS
impl JsTransactionContext {
    pub(crate) fn new(inner: JsBaseQueryable, tx_ctx_proxy: TransactionContextProxy) -> Self {
        Self { inner, tx_ctx_proxy }
    }

    pub fn start_transaction(&self) -> impl Future<Output = quaint::Result<Box<JsTransaction>>> + '_ {
        UnsafeFuture(self.tx_ctx_proxy.start_transaction())
    }
}

#[async_trait]
impl Queryable for JsTransactionContext {
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

    async fn describe_query(&self, sql: &str) -> quaint::Result<DescribedQuery> {
        self.inner.describe_query(sql).await
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

// Wrapper around JS transaction objects that implements Queryable
// and quaint::Transaction. Can be used in place of quaint transaction,
// but delegates most operations to JS
pub(crate) struct JsTransaction {
    tx_proxy: TransactionProxy,
    inner: JsBaseQueryable,
}

impl JsTransaction {
    pub(crate) fn new(inner: JsBaseQueryable, tx_proxy: TransactionProxy) -> Self {
        Self { inner, tx_proxy }
    }

    pub fn options(&self) -> &TransactionOptions {
        self.tx_proxy.options()
    }

    pub async fn raw_phantom_cmd(&self, cmd: &str) -> quaint::Result<()> {
        let params = &[];
        quaint::connector::metrics::query(
            "js.raw_phantom_cmd",
            self.inner.db_system_name,
            cmd,
            params,
            move || async move { Ok(()) },
        )
        .await
    }
}

#[async_trait]
impl QuaintTransaction for JsTransaction {
    async fn commit(&self) -> quaint::Result<()> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let commit_stmt = "COMMIT";

        if self.options().use_phantom_query {
            let commit_stmt = JsBaseQueryable::phantom_query_message(commit_stmt);
            self.raw_phantom_cmd(commit_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(commit_stmt).await?;
        }

        UnsafeFuture(self.tx_proxy.commit()).await
    }

    async fn rollback(&self) -> quaint::Result<()> {
        // increment of this gauge is done in DriverProxy::startTransaction
        decrement_gauge!("prisma_client_queries_active", 1.0);

        let rollback_stmt = "ROLLBACK";

        if self.options().use_phantom_query {
            let rollback_stmt = JsBaseQueryable::phantom_query_message(rollback_stmt);
            self.raw_phantom_cmd(rollback_stmt.as_str()).await?;
        } else {
            self.inner.raw_cmd(rollback_stmt).await?;
        }

        UnsafeFuture(self.tx_proxy.rollback()).await
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

#[cfg(target_arch = "wasm32")]
impl super::wasm::FromJsValue for JsTransactionContext {
    fn from_js_value(value: wasm_bindgen::prelude::JsValue) -> JsResult<Self> {
        use wasm_bindgen::JsCast;

        let object = value.dyn_into::<JsObject>()?;
        let common_proxy = CommonProxy::new(&object)?;
        let base = JsBaseQueryable::new(common_proxy);
        let tx_ctx_proxy = TransactionContextProxy::new(&object)?;

        Ok(Self::new(base, tx_ctx_proxy))
    }
}

/// Implementing unsafe `from_napi_value` allows retrieving a threadsafe `JsTransactionContext` in `DriverProxy`
/// while keeping derived futures `Send`.
#[cfg(not(target_arch = "wasm32"))]
impl ::napi::bindgen_prelude::FromNapiValue for JsTransactionContext {
    unsafe fn from_napi_value(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> JsResult<Self> {
        let object = JsObject::from_napi_value(env, napi_val)?;
        let common_proxy = CommonProxy::new(&object)?;
        let tx_ctx_proxy = TransactionContextProxy::new(&object)?;

        Ok(Self::new(JsBaseQueryable::new(common_proxy), tx_ctx_proxy))
    }
}
