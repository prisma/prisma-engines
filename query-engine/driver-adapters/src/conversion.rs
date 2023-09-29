use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};
use quaint::ast::Value as QuaintValue;
use serde::Serialize;
use serde_json::value::Value as JsonValue;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum JSArg {
    RawString(String),
    Value(serde_json::Value),
    Buffer(Vec<u8>),
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
            JSArg::Buffer(bytes) => ToNapiValue::to_napi_value(
                env,
                napi::Env::from_raw(env).create_buffer_with_data(bytes)?.into_raw(),
            ),
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
            quaint_value => JSArg::from(JsonValue::from(quaint_value.clone())),
        };

        values.push(res);
    }

    Ok(values)
}
