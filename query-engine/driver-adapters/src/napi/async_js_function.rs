use std::marker::PhantomData;

use napi::{
    bindgen_prelude::*,
    threadsafe_function::{ErrorStrategy, ThreadsafeFunction},
};

use super::{
    error::{async_unwinding_panic, into_quaint_error},
    result::JsResult,
};

/// Wrapper for napi-rs's ThreadsafeFunction that is aware of
/// JS drivers conventions. Performs following things:
/// - Automatically unrefs the function so it won't hold off event loop
/// - Awaits for returned Promise
/// - Unpacks JS `Result` type into Rust `Result` type and converts the error
/// into `quaint::Error`.
/// - Catches panics and converts them to `quaint:Error`
pub(crate) struct AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: ToNapiValue + 'static,
    ReturnType: FromNapiValue + 'static,
{
    threadsafe_fn: ThreadsafeFunction<ArgType, ErrorStrategy::Fatal>,
    _phantom: PhantomData<ReturnType>,
}

impl<ArgType, ReturnType> AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: ToNapiValue + 'static,
    ReturnType: FromNapiValue + 'static,
{
    fn from_threadsafe_function(
        mut threadsafe_fn: ThreadsafeFunction<ArgType, ErrorStrategy::Fatal>,
        env: Env,
    ) -> napi::Result<Self> {
        threadsafe_fn.unref(&env)?;

        Ok(AsyncJsFunction {
            threadsafe_fn,
            _phantom: PhantomData,
        })
    }

    pub(crate) async fn call(&self, arg: ArgType) -> quaint::Result<ReturnType> {
        let js_result = async_unwinding_panic(async {
            let promise = self
                .threadsafe_fn
                .call_async::<Promise<JsResult<ReturnType>>>(arg)
                .await?;
            promise.await
        })
        .await
        .map_err(into_quaint_error)?;
        js_result.into()
    }

    pub(crate) fn as_raw(&self) -> &ThreadsafeFunction<ArgType, ErrorStrategy::Fatal> {
        &self.threadsafe_fn
    }
}

impl<ArgType, ReturnType> FromNapiValue for AsyncJsFunction<ArgType, ReturnType>
where
    ArgType: ToNapiValue + 'static,
    ReturnType: FromNapiValue + 'static,
{
    unsafe fn from_napi_value(napi_env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        let env = Env::from_raw(napi_env);
        let threadsafe_fn = ThreadsafeFunction::from_napi_value(napi_env, napi_val)?;
        Self::from_threadsafe_function(threadsafe_fn, env)
    }
}
