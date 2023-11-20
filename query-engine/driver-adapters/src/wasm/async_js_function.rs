use js_sys::{Function as JsFunction, Promise as JsPromise};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::describe::WasmDescribe;
use wasm_bindgen::{JsError, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::error::into_quaint_error;

type JsResult<T> = core::result::Result<T, JsValue>;

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

impl<T, R> AsyncJsFunction<T, R>
where
    T: Serialize,
    R: DeserializeOwned,
{
    pub async fn call(&self, arg1: T) -> quaint::Result<R> {
        let call_internal = async {
            let arg1 = serde_wasm_bindgen::to_value(&arg1).map_err(|err| JsError::from(&err))?;
            let promise = self.threadsafe_fn.call1(&JsValue::null(), &arg1)?;
            let future = JsFuture::from(JsPromise::from(promise));
            let value = future.await?;
            serde_wasm_bindgen::from_value(value).map_err(|err| JsValue::from(err))
        };

        match call_internal.await {
            Ok(result) => Ok(result),
            Err(err) => Err(into_quaint_error(err)),
        }
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
