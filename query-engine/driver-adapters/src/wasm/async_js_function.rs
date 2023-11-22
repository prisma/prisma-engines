use js_sys::{Function as JsFunction, Promise as JsPromise};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::describe::WasmDescribe;
use wasm_bindgen::{JsError, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::error::into_quaint_error;
use super::result::JsResult;

#[derive(Clone, Default)]
pub(crate) struct AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: DeserializeOwned,
{
    pub threadsafe_fn: JsFunction,

    _phantom_arg: PhantomData<ArgType>,
    _phantom_return: PhantomData<ReturnType>,
}

impl<T, R> From<JsFunction> for AsyncJsFunction<T, R>
where
    T: Serialize,
    R: DeserializeOwned,
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
    R: DeserializeOwned,
{
    pub async fn call(&self, arg1: T) -> quaint::Result<R> {
        let result = self.call_internal(arg1).await;

        match result {
            Ok(js_result) => js_result.into(),
            Err(err) => Err(into_quaint_error(err)),
        }
    }

    async fn call_internal(&self, arg1: T) -> Result<JsResult<R>, JsValue> {
        let arg1 = serde_wasm_bindgen::to_value(&arg1).map_err(|err| JsValue::from(JsError::from(&err)))?;
        let promise = self.threadsafe_fn.call1(&JsValue::null(), &arg1)?;
        let future = JsFuture::from(JsPromise::from(promise));
        let value = future.await?;
        let js_result: JsResult<R> = value.try_into()?;

        Ok(js_result)
    }
}

impl<ArgType, ReturnType> WasmDescribe for AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: DeserializeOwned,
{
    fn describe() {
        JsFunction::describe();
    }
}

impl<ArgType, ReturnType> FromWasmAbi for AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: DeserializeOwned,
{
    type Abi = <JsFunction as FromWasmAbi>::Abi;

    unsafe fn from_abi(js: Self::Abi) -> Self {
        Self {
            threadsafe_fn: JsFunction::from_abi(js),
            _phantom_arg: PhantomData::<ArgType> {},
            _phantom_return: PhantomData::<ReturnType> {},
        }
    }
}
