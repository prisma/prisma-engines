use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};
use napi::NapiValue;
use quaint::ast::Value as QuaintValue;
use serde::Serialize;
use serde_json::value::Value as JsonValue;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum JSArg {
    RawString(String),
    Value(serde_json::Value),
    Buffer(Vec<u8>),
    Array(Vec<JSArg>),
}

impl From<JsonValue> for JSArg {
    fn from(v: JsonValue) -> Self {
        JSArg::Value(v)
    }
}

// FromNapiValue is the napi equivalent to serde::Deserialize.
// Note: we can safely leave this unimplemented as we don't need deserialize napi_value back to JSArg.
// However, removing this altogether would cause a compile error.
impl FromNapiValue for JSArg {
    unsafe fn from_napi_value(_env: napi::sys::napi_env, _napi_value: napi::sys::napi_value) -> napi::Result<Self> {
        unreachable!()
    }
}

// ToNapiValue is the napi equivalent to serde::Serialize.
impl ToNapiValue for JSArg {
    unsafe fn to_napi_value(env: napi::sys::napi_env, value: Self) -> napi::Result<napi::sys::napi_value> {
        match value {
            JSArg::RawString(s) => ToNapiValue::to_napi_value(env, s),
            JSArg::Value(v) => ToNapiValue::to_napi_value(env, v),
            JSArg::Buffer(bytes) => {
                ToNapiValue::to_napi_value(env, napi::Env::from_raw(env).create_buffer_with_data(bytes)?.into_raw())
            }
            // While arrays are encodable as JSON generally, their element might not be, or may be
            // represented in a different way than we need. We use this custom logic for all arrays
            // to avoid having separate `JsonArray` and `BytesArray` variants in `JSArg` and
            // avoid complicating the logic in `conv_params`.
            JSArg::Array(items) => {
                let env = napi::Env::from_raw(env);
                let mut array = env.create_array(items.len().try_into().expect("JS array length must fit into u32"))?;

                for (index, item) in items.into_iter().enumerate() {
                    let js_value = ToNapiValue::to_napi_value(env.raw(), item)?;
                    // TODO: NapiRaw could be implemented for sys::napi_value directly, there should
                    // be no need for re-wrapping; submit a patch to napi-rs and simplify here.
                    array.set(index as u32, napi::JsUnknown::from_raw_unchecked(env.raw(), js_value))?;
                }

                ToNapiValue::to_napi_value(env.raw(), array)
            }
        }
    }
}

pub fn conv_params(params: &[QuaintValue<'_>]) -> serde_json::Result<Vec<JSArg>> {
    let mut values = Vec::with_capacity(params.len());

    for pv in params {
        let res = match pv {
            QuaintValue::Json(s) => match s {
                Some(ref s) => {
                    let json_str = serde_json::to_string(s)?;
                    JSArg::RawString(json_str)
                }
                None => JsonValue::Null.into(),
            },
            QuaintValue::Bytes(bytes) => match bytes {
                Some(bytes) => JSArg::Buffer(bytes.to_vec()),
                None => JsonValue::Null.into(),
            },
            quaint_value @ QuaintValue::Numeric(bd) => match bd {
                Some(bd) => match bd.to_string().parse::<f64>() {
                    Ok(double) => JSArg::from(JsonValue::from(double)),
                    Err(_) => JSArg::from(JsonValue::from(quaint_value.clone())),
                },
                None => JsonValue::Null.into(),
            },
            QuaintValue::Array(Some(items)) => JSArg::Array(conv_params(items)?),
            quaint_value => JSArg::from(JsonValue::from(quaint_value.clone())),
        };

        values.push(res);
    }

    Ok(values)
}
