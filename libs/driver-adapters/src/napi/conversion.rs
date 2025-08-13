use crate::conversion::MaybeDefined;
pub(crate) use crate::conversion::{JSArg, JSArgType};

use napi::NapiValue;
use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};

// FromNapiValue is the napi equivalent to serde::Deserialize.
// Note: we can safely leave this unimplemented as we don't need deserialize napi_value back to JSArg.
// However, removing this altogether would cause a compile error.
impl FromNapiValue for JSArg {
    unsafe fn from_napi_value(_env: napi::sys::napi_env, _napi_value: napi::sys::napi_value) -> napi::Result<Self> {
        unreachable!()
    }
}

impl FromNapiValue for JSArgType {
    unsafe fn from_napi_value(_env: napi::sys::napi_env, _napi_value: napi::sys::napi_value) -> napi::Result<Self> {
        unreachable!()
    }
}

// ToNapiValue is the napi equivalent to serde::Serialize.
impl ToNapiValue for JSArg {
    unsafe fn to_napi_value(env: napi::sys::napi_env, value: Self) -> napi::Result<napi::sys::napi_value> {
        match value {
            JSArg::SafeInt(v) => unsafe { ToNapiValue::to_napi_value(env, v) },
            JSArg::Value(v) => unsafe { ToNapiValue::to_napi_value(env, v) },
            JSArg::Buffer(bytes) => {
                let env = unsafe { napi::Env::from_raw(env) };
                let length = bytes.len();
                let buffer = env.create_arraybuffer_with_data(bytes)?.into_raw();
                let byte_array = buffer.into_typedarray(napi::TypedArrayType::Uint8, length, 0)?;

                unsafe { ToNapiValue::to_napi_value(env.raw(), byte_array) }
            }
            // While arrays are encodable as JSON generally, their element might not be, or may be
            // represented in a different way than we need. We use this custom logic for all arrays
            // to avoid having separate `JsonArray` and `BytesArray` variants in `JSArg` and
            // avoid complicating the logic in `conv_params`.
            JSArg::Array(items) => {
                let env = unsafe { napi::Env::from_raw(env) };
                let mut array = env.create_array(items.len().try_into().expect("JS array length must fit into u32"))?;

                for (index, item) in items.into_iter().enumerate() {
                    let js_value = unsafe { ToNapiValue::to_napi_value(env.raw(), item) }?;
                    // TODO: NapiRaw could be implemented for sys::napi_value directly, there should
                    //  be no need for re-wrapping; submit a patch to napi-rs and simplify here.
                    array.set(index as u32, unsafe {
                        napi::JsUnknown::from_raw_unchecked(env.raw(), js_value)
                    })?;
                }

                unsafe { ToNapiValue::to_napi_value(env.raw(), array) }
            }
        }
    }
}

impl ToNapiValue for JSArgType {
    unsafe fn to_napi_value(env: napi::sys::napi_env, value: Self) -> napi::Result<napi::sys::napi_value> {
        let env = unsafe { napi::Env::from_raw(env) };

        let mut obj = env.create_object()?;
        obj.set_named_property("scalarType", <&str>::from(value.scalar_type))?;
        obj.set_named_property("dbType", MaybeDefined(value.db_type))?;
        obj.set_named_property("arity", <&str>::from(value.arity))?;

        unsafe { ToNapiValue::to_napi_value(env.raw(), obj) }
    }
}

impl<V: ToNapiValue> ToNapiValue for MaybeDefined<V> {
    unsafe fn to_napi_value(env: napi::sys::napi_env, val: Self) -> napi::Result<napi::sys::napi_value> {
        match val {
            MaybeDefined(Some(v)) => unsafe { ToNapiValue::to_napi_value(env, v) },
            MaybeDefined(None) => {
                let undefined = unsafe { napi::Env::from_raw(env) }.get_undefined()?;
                unsafe { ToNapiValue::to_napi_value(env, undefined) }
            }
        }
    }
}
