use napi::{bindgen_prelude::FromNapiValue, Env, JsUnknown, NapiValue};
use quaint::error::Error as QuaintError;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "kind")]
/// Wrapper for JS-side errors
/// See js-connectors/js-connector-utils/types file for example
pub(crate) enum JsConnectorError {
    /// Unexpected JS exception
    JsError { id: i32 },
    // in the future, expected errors that map to known user errors with PXXX codes will also go here
}

impl FromNapiValue for JsConnectorError {
    unsafe fn from_napi_value(napi_env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        let env = Env::from_raw(napi_env);
        let value = JsUnknown::from_raw(napi_env, napi_val)?;
        env.from_js_value(value)
    }
}

impl From<JsConnectorError> for QuaintError {
    fn from(value: JsConnectorError) -> Self {
        match value {
            JsConnectorError::JsError { id } => QuaintError::external_error(id),
        }
    }
}

/// Wrapper for JS-side result type
/// See js-connectors/js-connector-utils/types file for example
pub(crate) enum JsResult<T>
where
    T: FromNapiValue,
{
    Ok(T),
    Err(JsConnectorError),
}

impl<T> FromNapiValue for JsResult<T>
where
    T: FromNapiValue,
{
    unsafe fn from_napi_value(napi_env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        let value = JsUnknown::from_raw(napi_env, napi_val)?.coerce_to_object()?;
        let ok: bool = value.get_named_property("ok")?;
        if ok {
            let result_prop: JsUnknown = value.get_named_property("result")?;
            return Ok(Self::Ok(T::from_unknown(result_prop)?));
        }

        let error = value.get_named_property("error")?;
        Ok(Self::Err(error))
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
