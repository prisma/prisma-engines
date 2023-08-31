use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};
use quaint::ast::Value as QuaintValue;
use serde::Serialize;
use serde_json::value::Value as JsonValue;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum JSArg {
    RawString(String),
    Value(serde_json::Value),
}

impl From<JsonValue> for JSArg {
    fn from(v: JsonValue) -> Self {
        JSArg::Value(v)
    }
}

// FromNapiValue is the napi equivalent to serde::Deserialize.
// Note: we don't need to deserialize JSArg back to napi_value, so we can safely leave this unimplemented.
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
            quaint_value => {
                let json: JsonValue = quaint_value.clone().into();
                json.into()
            }
        };

        values.push(res);
    }

    Ok(values)
}
