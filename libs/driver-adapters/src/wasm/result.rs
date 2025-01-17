use js_sys::Boolean as JsBoolean;
use wasm_bindgen::{JsCast, JsValue};

use super::from_js::FromJsValue;
use crate::{error::DriverAdapterError, JsObjectExtern};

/// Wrapper for JS-side result type.
/// This Wasm-specific implementation has the same shape and API as the Napi implementation,
/// but it asks for a `FromJsValue` bound on the generic type.
/// The duplication is needed as it's currently impossible to have target-specific generic bounds in Rust.
pub(crate) enum AdapterResult<T>
where
    T: FromJsValue,
{
    Ok(T),
    Err(DriverAdapterError),
}

impl<T> FromJsValue for AdapterResult<T>
where
    T: FromJsValue,
{
    fn from_js_value(unknown: JsValue) -> Result<Self, JsValue> {
        let object = unknown.unchecked_into::<JsObjectExtern>();

        let ok: JsBoolean = object.get("ok".into())?.unchecked_into();
        let ok = ok.value_of();

        if ok {
            let js_value: JsValue = object.get("value".into())?;
            let deserialized = T::from_js_value(js_value)?;
            return Ok(Self::Ok(deserialized));
        }

        let error = object.get("error".into())?;
        let error: DriverAdapterError = serde_wasm_bindgen::from_value(error)?;
        Ok(Self::Err(error))
    }
}

impl<T> From<AdapterResult<T>> for quaint::Result<T>
where
    T: FromJsValue,
{
    fn from(value: AdapterResult<T>) -> Self {
        match value {
            AdapterResult::Ok(result) => Ok(result),
            AdapterResult::Err(error) => Err(error.into()),
        }
    }
}
