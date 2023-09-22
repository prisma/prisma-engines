use napi::{bindgen_prelude::FromNapiValue, Env, JsUnknown, NapiValue};
use quaint::error::{Error as QuaintError, PostgresError};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(remote = "PostgresError")]
pub struct PostgresErrorDef {
    code: String,
    message: String,
    severity: String,
    detail: Option<String>,
    column: Option<String>,
    hint: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
/// Wrapper for JS-side errors
/// See driver-adapters/js/adapter-utils/src/types.ts file for example
pub(crate) enum DriverAdapterError {
    /// Unexpected JS exception
    GenericJsError {
        id: i32,
    },

    PostgresError(#[serde(with = "PostgresErrorDef")] PostgresError),
    // in the future, expected errors that map to known user errors with PXXX codes will also go here
}

impl FromNapiValue for DriverAdapterError {
    unsafe fn from_napi_value(napi_env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        let env = Env::from_raw(napi_env);
        let value = JsUnknown::from_raw(napi_env, napi_val)?;
        env.from_js_value(value)
    }
}

impl From<DriverAdapterError> for QuaintError {
    fn from(value: DriverAdapterError) -> Self {
        match value {
            DriverAdapterError::GenericJsError { id } => QuaintError::external_error(id),
            DriverAdapterError::PostgresError(e) => e.into(),
            // in future, more error types would be added and we'll need to convert them to proper QuaintErrors here
        }
    }
}

/// Wrapper for JS-side result type
/// See driver-adapters/js/adapter-utils/src/types.ts file for example
pub(crate) enum JsResult<T>
where
    T: FromNapiValue,
{
    Ok(T),
    Err(DriverAdapterError),
}

impl<T> JsResult<T>
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

impl<T> FromNapiValue for JsResult<T>
where
    T: FromNapiValue,
{
    unsafe fn from_napi_value(napi_env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        Self::from_js_unknown(JsUnknown::from_raw(napi_env, napi_val)?)
    }
}

impl<T> From<JsResult<T>> for quaint::Result<T>
where
    T: FromNapiValue,
{
    fn from(value: JsResult<T>) -> Self {
        match value {
            JsResult::Ok(result) => Ok(result),
            JsResult::Err(error) => Err(error.into()),
        }
    }
}
