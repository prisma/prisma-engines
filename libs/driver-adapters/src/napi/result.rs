use crate::error::DriverAdapterError;
use napi::{
    Env, JsUnknown, NapiValue,
    bindgen_prelude::{FromNapiValue, TypeName, ValidateNapiValue},
};

impl FromNapiValue for DriverAdapterError {
    unsafe fn from_napi_value(napi_env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        unsafe {
            let env = Env::from_raw(napi_env);
            let value = JsUnknown::from_raw(napi_env, napi_val)?;
            env.from_js_value(value)
        }
    }
}

impl ValidateNapiValue for DriverAdapterError {}

impl TypeName for DriverAdapterError {
    fn type_name() -> &'static str {
        "DriverAdapterError"
    }

    fn value_type() -> napi::ValueType {
        napi::ValueType::Object
    }
}

/// Wrapper for JS-side result type.
/// This Napi-specific implementation has the same shape and API as the Wasm implementation,
/// but it asks for a `FromNapiValue` bound on the generic type.
/// The duplication is needed as it's currently impossible to have target-specific generic bounds in Rust.
pub(crate) enum AdapterResult<T>
where
    T: FromNapiValue,
{
    Ok(T),
    Err(DriverAdapterError),
}

impl<T> AdapterResult<T>
where
    T: FromNapiValue,
{
    fn from_js_unknown(unknown: JsUnknown) -> napi::Result<Self> {
        let object = unknown.coerce_to_object()?;
        let ok: bool = object.get_named_property("ok")?;
        if ok {
            let value: JsUnknown = object.get_named_property("value")?;
            return Ok(Self::Ok(T::from_unknown(value)?));
        }

        let error = object.get_named_property("error")?;
        Ok(Self::Err(error))
    }
}

impl<T> FromNapiValue for AdapterResult<T>
where
    T: FromNapiValue,
{
    unsafe fn from_napi_value(napi_env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        unsafe { Self::from_js_unknown(JsUnknown::from_raw(napi_env, napi_val)?) }
    }
}

impl<T> From<AdapterResult<T>> for quaint::Result<T>
where
    T: FromNapiValue,
{
    fn from(value: AdapterResult<T>) -> Self {
        match value {
            AdapterResult::Ok(result) => Ok(result),
            AdapterResult::Err(error) => Err(error.into()),
        }
    }
}
