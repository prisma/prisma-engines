use js_sys::Boolean as JsBoolean;
use wasm_bindgen::{JsCast, JsValue};

use super::from_js::FromJsValue;
use crate::{JsObjectExtern, error::DriverAdapterError};

/// Wrapper for JS-side result type.
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
