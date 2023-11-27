use js_sys::{Function as JsFunction, Promise as JsPromise};
use serde::Serialize;
use std::marker::PhantomData;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::describe::WasmDescribe;
use wasm_bindgen::{JsCast, JsError, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::error::into_quaint_error;
use super::from_js::FromJsValue;
use super::result::JsResult;

#[derive(Clone)]
pub(crate) struct AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: FromJsValue,
{
    threadsafe_fn: JsFunction,

    _phantom_arg: PhantomData<ArgType>,
    _phantom_return: PhantomData<ReturnType>,
}

impl<T, R> From<JsFunction> for AsyncJsFunction<T, R>
where
    T: Serialize,
    R: FromJsValue,
{
    fn from(js_fn: JsFunction) -> Self {
        Self {
            threadsafe_fn: js_fn,
            _phantom_arg: PhantomData::<T> {},
            _phantom_return: PhantomData::<R> {},
        }
    }
}

impl<T, R> AsyncJsFunction<T, R>
where
    T: Serialize,
    R: FromJsValue,
{
    pub(crate) async fn call(&self, arg1: T) -> quaint::Result<R> {
        let result = self.call_internal(arg1).await;

        match result {
            Ok(js_result) => js_result.into(),
            Err(err) => Err(into_quaint_error(err)),
        }
    }

    async fn call_internal(&self, arg1: T) -> Result<JsResult<R>, JsValue> {
        let arg1 = serde_wasm_bindgen::to_value(&arg1).map_err(|err| JsValue::from(JsError::from(&err)))?;
        let return_value = self.threadsafe_fn.call1(&JsValue::null(), &arg1)?;

        let value = if let Some(promise) = return_value.dyn_ref::<JsPromise>() {
            JsFuture::from(promise.to_owned()).await?
        } else {
            return_value
        };

        let js_result = JsResult::<R>::from_js_value(value)?;

        Ok(js_result)
    }

    pub(crate) fn call_non_blocking(&self, arg: T) {
        if let Ok(arg) = serde_wasm_bindgen::to_value(&arg) {
            _ = self.threadsafe_fn.call1(&JsValue::null(), &arg);
        }
    }
}

impl<ArgType, ReturnType> WasmDescribe for AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: FromJsValue,
{
    fn describe() {
        JsFunction::describe();
    }
}

impl<ArgType, ReturnType> FromWasmAbi for AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: FromJsValue,
{
    type Abi = <JsFunction as FromWasmAbi>::Abi;

    unsafe fn from_abi(js: Self::Abi) -> Self {
        JsFunction::from_abi(js).into()
    }
}
