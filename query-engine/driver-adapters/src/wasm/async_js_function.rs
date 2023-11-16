use js_sys::{Function as JsFunction, Object as JsObject, Promise as JsPromise};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use wasm_bindgen::{prelude::wasm_bindgen, JsError, JsValue};
use wasm_bindgen_futures::JsFuture;

type JsResult<T> = core::result::Result<T, JsValue>;

pub(crate) struct AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: Serialize + 'static,
    ReturnType: DeserializeOwned + 'static,
{
    threadsafe_fn: JsFunction,
    _phantom_arg: PhantomData<ArgType>,
    _phantom_return: PhantomData<ReturnType>,
}

impl<ArgType, ReturnType> AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: Serialize + 'static,
    ReturnType: DeserializeOwned + 'static,
{
    async fn call(&self, arg1: ArgType) -> JsResult<ReturnType> {
        let arg1 = serde_wasm_bindgen::to_value(&arg1).map_err(|err| JsError::from(&err))?;
        let promise = self.threadsafe_fn.call1(&JsValue::null(), &arg1)?;
        let future = JsFuture::from(JsPromise::from(promise));
        let value = future.await?;
        serde_wasm_bindgen::from_value(value).map_err(|err| JsValue::from(err))
    }
}
