#![allow(dead_code)]
#![allow(unused_variables)]

// This code will likely live in a separate crate, but for now it's here.

use async_trait::async_trait;
use js_sys::{Function as JsFunction, JsString, Object as JsObject, Promise as JsPromise, Reflect as JsReflect};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::{JsCast, JsValue};

type Result<T> = std::result::Result<T, js_sys::Error>;

pub struct CommonProxy {
    /// Execute a query given as SQL, interpolating the given parameters.
    query_raw: JsFunction,

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    execute_raw: JsFunction,

    /// Return the flavour for this driver.
    pub(crate) flavour: String,
}

impl CommonProxy {
    pub(crate) fn new(driver: &JsObject) -> Result<Self> {
        let query_raw = JsReflect::get(driver, &"queryRaw".into())?.dyn_into::<JsFunction>()?;
        let execute_raw = JsReflect::get(driver, &"executeRaw".into())?.dyn_into::<JsFunction>()?;
        let flavour: String = JsReflect::get(driver, &"flavour".into())?
            .dyn_into::<JsString>()?
            .into();

        let common_proxy = Self {
            query_raw,
            execute_raw,
            flavour,
        };
        Ok(common_proxy)
    }
}

pub struct DriverProxy {
    start_transaction: JsFunction,
}

impl DriverProxy {
    pub(crate) fn new(driver: &JsObject) -> Result<Self> {
        let start_transaction = JsReflect::get(driver, &"startTransaction".into())?.dyn_into::<JsFunction>()?;

        let driver_proxy = Self { start_transaction };
        Ok(driver_proxy)
    }
}

pub struct JsQueryable {
    inner: CommonProxy,
    driver_proxy: DriverProxy,
}

impl JsQueryable {
    pub fn new(inner: CommonProxy, driver_proxy: DriverProxy) -> Self {
        Self { inner, driver_proxy }
    }
}

pub fn from_wasm(driver: JsObject) -> Result<JsQueryable> {
    let common_proxy = CommonProxy::new(&driver)?;
    let driver_proxy = DriverProxy::new(&driver)?;

    let js_queryable = JsQueryable::new(common_proxy, driver_proxy);
    Ok(js_queryable)
}

#[async_trait(?Send)]
trait JsAsyncFunc {
    async fn call1_async<T, R>(&self, arg1: T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned;

    fn call0_sync<R>(&self) -> Result<R>
    where
        R: DeserializeOwned;
}

#[async_trait(?Send)]
impl JsAsyncFunc for JsFunction {
    async fn call1_async<T, R>(&self, arg1: T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let arg1 = serde_wasm_bindgen::to_value(&arg1).map_err(|err| js_sys::Error::new(&err.to_string()))?;
        let promise = self.call1(&JsValue::null(), &arg1)?;
        let future = wasm_bindgen_futures::JsFuture::from(JsPromise::from(promise));
        let value = future.await?;
        serde_wasm_bindgen::from_value(value).map_err(|err| js_sys::Error::new(&err.to_string()))
    }

    fn call0_sync<R>(&self) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let value = self.call0(&JsValue::null())?;
        serde_wasm_bindgen::from_value(value).map_err(|err| js_sys::Error::new(&err.to_string()))
    }
}
