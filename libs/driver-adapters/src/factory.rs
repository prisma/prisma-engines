use std::sync::Arc;

use async_trait::async_trait;
use quaint::connector::{ExternalConnector, ExternalConnectorFactory};

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
            .map(|queryable| Arc::new(queryable) as Arc<dyn ExternalConnector>)
    }

    async fn connect_to_shadow_db(&self) -> Option<quaint::Result<Arc<dyn ExternalConnector>>> {
        self.connect_to_shadow_db()
            .await
            .map(|result| result.map(|queryable| Arc::new(queryable) as Arc<dyn ExternalConnector>))
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
