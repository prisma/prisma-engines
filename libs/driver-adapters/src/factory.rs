use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use async_trait::async_trait;
use quaint::{
    connector::{DescribedQuery, ExternalConnector, ExternalConnectorFactory, IsolationLevel, Transaction},
    prelude::{
        ExternalConnectionInfo, Query as QuaintQuery, Queryable as QuaintQueryable, ResultSet, TransactionCapable,
    },
};

use crate::proxy::AdapterFactoryProxy;
use crate::queryable::JsQueryable;
use crate::{JsObject, JsResult};

pub struct JsAdapterFactory {
    inner: AdapterFactoryProxy,
}

impl JsAdapterFactory {
    pub(crate) fn new(proxy: AdapterFactoryProxy) -> Self {
        Self { inner: proxy }
    }

    pub async fn connect(&self) -> quaint::Result<JsQueryable> {
        self.inner.connect().await
    }

    pub async fn connect_to_shadow_db(&self) -> Option<quaint::Result<JsQueryable>> {
        self.inner.connect_to_shadow_db().await
    }
}

#[async_trait]
impl ExternalConnectorFactory for JsAdapterFactory {
    async fn connect(&self) -> quaint::Result<Arc<dyn ExternalConnector>> {
        self.connect()
            .await
            .map(|queryable| Arc::new(JsQueryableDropGuard::new(queryable)) as Arc<dyn ExternalConnector>)
    }

    async fn connect_to_shadow_db(&self) -> Option<quaint::Result<Arc<dyn ExternalConnector>>> {
        self.connect_to_shadow_db().await.map(|result| {
            result.map(|queryable| Arc::new(JsQueryableDropGuard::new(queryable)) as Arc<dyn ExternalConnector>)
        })
    }
}

/// A wrapper around `JsQueryable` that ensures that the dispose method is called after use.
/// The user can still call `dispose` and await on it to ensure a timely release of resources.
#[derive(Debug)]
struct JsQueryableDropGuard {
    inner: JsQueryable,
    disposed: AtomicBool,
}

impl JsQueryableDropGuard {
    fn new(inner: JsQueryable) -> Self {
        Self {
            inner,
            disposed: AtomicBool::new(false),
        }
    }
}

impl Drop for JsQueryableDropGuard {
    fn drop(&mut self) {
        if !self.disposed.swap(true, Ordering::Relaxed) {
            self.inner.dispose_non_blocking();
        }
    }
}

#[async_trait]
impl ExternalConnector for JsQueryableDropGuard {
    async fn execute_script(&self, script: &str) -> quaint::Result<()> {
        self.inner.execute_script(script).await
    }

    async fn get_connection_info(&self) -> quaint::Result<ExternalConnectionInfo> {
        self.inner.get_connection_info().await
    }

    async fn dispose(&self) -> quaint::Result<()> {
        if !self.disposed.swap(true, Ordering::Relaxed) {
            self.inner.dispose().await
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl TransactionCapable for JsQueryableDropGuard {
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> quaint::Result<Box<dyn Transaction + 'a>> {
        self.inner.start_transaction(isolation).await
    }
}

#[async_trait]
impl QuaintQueryable for JsQueryableDropGuard {
    async fn query(&self, q: QuaintQuery<'_>) -> quaint::Result<ResultSet> {
        self.inner.query(q).await
    }

    async fn query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<ResultSet> {
        self.inner.query_raw_typed(sql, params).await
    }

    async fn describe_query(&self, sql: &str) -> quaint::Result<DescribedQuery> {
        self.inner.describe_query(sql).await
    }

    async fn execute(&self, q: QuaintQuery<'_>) -> quaint::Result<u64> {
        self.inner.execute(q).await
    }

    async fn execute_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<u64> {
        self.inner.execute_raw(sql, params).await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<u64> {
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
impl super::wasm::FromJsValue for JsAdapterFactory {
    fn from_js_value(value: wasm_bindgen::prelude::JsValue) -> JsResult<Self> {
        use wasm_bindgen::JsCast;

        let object = value.dyn_into::<JsObject>()?;
        let common_proxy = AdapterFactoryProxy::new(&object)?;
        Ok(Self::new(common_proxy))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ::napi::bindgen_prelude::FromNapiValue for JsAdapterFactory {
    unsafe fn from_napi_value(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> JsResult<Self> {
        let object = JsObject::from_napi_value(env, napi_val)?;
        let common_proxy = AdapterFactoryProxy::new(&object)?;
        Ok(Self::new(common_proxy))
    }
}
