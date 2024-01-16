use js_sys::{Function as JsFunction, Promise as JsPromise};
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use std::marker::PhantomData;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::describe::WasmDescribe;
use wasm_bindgen::{JsCast, JsError, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::error::into_quaint_error;
use super::from_js::FromJsValue;
use crate::AdapterResult;

// - `serialize_missing_as_null` is required to make sure that "empty" values (e.g., `None` and `()`)
//   are serialized as `null` and not `undefined`.
//   This is due to certain drivers (e.g., LibSQL) not supporting `undefined` values.
// - `serialize_large_number_types_as_bigints` is required to allow reading bigints from Prisma Client.
pub(crate) static SERIALIZER: Serializer = Serializer::new()
    .serialize_large_number_types_as_bigints(true)
    .serialize_missing_as_null(true);

#[derive(Clone)]
pub(crate) struct AdapterMethod<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: FromJsValue,
{
    fn_: JsFunction,

    _phantom_arg: PhantomData<ArgType>,
    _phantom_return: PhantomData<ReturnType>,
}

impl<T, R> From<JsValue> for AdapterMethod<T, R>
where
    T: Serialize,
    R: FromJsValue,
{
    fn from(js_value: JsValue) -> Self {
        JsFunction::from(js_value).into()
    }
}

impl<T, R> From<JsFunction> for AdapterMethod<T, R>
where
    T: Serialize,
    R: FromJsValue,
{
    fn from(js_fn: JsFunction) -> Self {
        Self {
            fn_: js_fn,
            _phantom_arg: PhantomData::<T> {},
            _phantom_return: PhantomData::<R> {},
        }
    }
}

impl<T, R> AdapterMethod<T, R>
where
    T: Serialize,
    R: FromJsValue,
{
    pub(crate) async fn call_as_async(&self, arg1: T) -> quaint::Result<R> {
        let future = self
            .call_internal(arg1)
            .await
            .and_then(|v| v.dyn_into::<JsPromise>())
            .map(JsFuture::from)
            .map_err(into_quaint_error)?;

        let return_value = future.await.map_err(into_quaint_error)?;
        Self::js_result_into_quaint_result(return_value)
    }

    pub(crate) async fn call_as_sync(&self, arg1: T) -> quaint::Result<R> {
        let return_value = self.call_internal(arg1).await.map_err(into_quaint_error)?;

        Self::js_result_into_quaint_result(return_value)
    }

    fn js_result_into_quaint_result(value: JsValue) -> quaint::Result<R> {
        AdapterResult::<R>::from_js_value(value)
            .map_err(into_quaint_error)?
            .into()
    }

    async fn call_internal(&self, arg1: T) -> Result<JsValue, JsValue> {
        let arg1 = arg1
            .serialize(&SERIALIZER)
            .map_err(|err| JsValue::from(JsError::from(&err)))?;
        self.fn_.call1(&JsValue::null(), &arg1)
    }

    pub(crate) fn call_non_blocking(&self, arg: T) {
        if let Ok(arg) = serde_wasm_bindgen::to_value(&arg) {
            _ = self.fn_.call1(&JsValue::null(), &arg);
        }
    }
}

impl<ArgType, ReturnType> WasmDescribe for AdapterMethod<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: FromJsValue,
{
    fn describe() {
        JsFunction::describe();
    }
}

impl<ArgType, ReturnType> FromWasmAbi for AdapterMethod<ArgType, ReturnType>
where
    ArgType: Serialize,
    ReturnType: FromJsValue,
{
    type Abi = <JsFunction as FromWasmAbi>::Abi;

    unsafe fn from_abi(js: Self::Abi) -> Self {
        JsFunction::from_abi(js).into()
    }
}
