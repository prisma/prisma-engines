use std::marker::PhantomData;

use napi::{
    bindgen_prelude::*,
    threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
};

use super::error::{async_unwinding_panic, into_quaint_error};
use crate::AdapterResult;

/// Wrapper for napi-rs's ThreadsafeFunction that is aware of
/// JS drivers conventions. Performs following things:
/// - Automatically unrefs the function so it won't hold off event loop
/// - Awaits for returned Promise
/// - Unpacks JS `Result` type into Rust `Result` type and converts the error
///   into `quaint::Error`.
/// - Catches panics and converts them to `quaint:Error`
pub(crate) struct AdapterMethod<ArgType, ReturnType>
where
    ArgType: ToNapiValue + 'static,
    ReturnType: FromNapiValue + 'static,
{
    threadsafe_fn: ThreadsafeFunction<ArgType, ErrorStrategy::Fatal>,
    _phantom: PhantomData<ReturnType>,
}

impl<ArgType, ReturnType> AdapterMethod<ArgType, ReturnType>
where
    ArgType: ToNapiValue + 'static,
    ReturnType: FromNapiValue + 'static,
{
    fn from_threadsafe_function(
        mut threadsafe_fn: ThreadsafeFunction<ArgType, ErrorStrategy::Fatal>,
        env: Env,
    ) -> napi::Result<Self> {
        threadsafe_fn.unref(&env)?;

        Ok(AdapterMethod {
            threadsafe_fn,
            _phantom: PhantomData,
        })
    }

    pub(crate) async fn call_as_async(&self, arg: ArgType) -> quaint::Result<ReturnType> {
        let js_result = async_unwinding_panic(async {
            let promise = self
                .threadsafe_fn
                .call_async::<Promise<AdapterResult<ReturnType>>>(arg)
                .await?;
            promise.await
        })
        .await
        .map_err(into_quaint_error)?;
        js_result.into()
    }

    pub(crate) async fn call_as_sync(&self, arg: ArgType) -> quaint::Result<ReturnType> {
        let js_result = self
            .threadsafe_fn
            .call_async::<AdapterResult<ReturnType>>(arg)
            .await
            .map_err(into_quaint_error)?;
        js_result.into()
    }

    pub(crate) fn call_non_blocking(&self, arg: ArgType) {
        _ = self.threadsafe_fn.call(arg, ThreadsafeFunctionCallMode::NonBlocking);
    }
}

impl<ArgType, ReturnType> FromNapiValue for AdapterMethod<ArgType, ReturnType>
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

impl<ArgType, ReturnType> ValidateNapiValue for AdapterMethod<ArgType, ReturnType>
where
    ArgType: ToNapiValue + 'static,
    ReturnType: FromNapiValue + 'static,
{
}

impl<ArgType, ReturnType> TypeName for AdapterMethod<ArgType, ReturnType>
where
    ArgType: ToNapiValue + 'static,
    ReturnType: FromNapiValue + 'static,
{
    fn type_name() -> &'static str {
        "AdapterMethod"
    }

    fn value_type() -> ValueType {
        ValueType::Function
    }
}
