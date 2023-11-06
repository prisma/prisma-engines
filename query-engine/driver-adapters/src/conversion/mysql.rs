use crate::conversion::JSArg;
use serde_json::value::Value as JsonValue;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const TIME_FORMAT: &str = "%H:%M:%S";

pub fn value_to_js_arg(value: &quaint::Value) -> serde_json::Result<JSArg> {
    let res = match &value.typed {
        quaint::ValueType::Json(s) => match s {
            Some(ref s) => {
                let json_str = serde_json::to_string(s)?;
                JSArg::RawString(json_str)
            }
            None => JsonValue::Null.into(),
        },
        quaint::ValueType::Bytes(bytes) => match bytes {
            Some(bytes) => JSArg::Buffer(bytes.to_vec()),
            None => JsonValue::Null.into(),
        },
        quaint::ValueType::Numeric(bd) => match bd {
            // converting decimal to string to preserve the precision
            Some(bd) => JSArg::RawString(bd.to_string()),
            None => JsonValue::Null.into(),
        },
        quaint::ValueType::DateTime(dt) => match dt {
            Some(dt) => JSArg::RawString(dt.format(DATE_FORMAT).to_string()),
            None => JsonValue::Null.into(),
        },
        quaint::ValueType::Time(dt) => match dt {
            Some(dt) => JSArg::RawString(dt.format(TIME_FORMAT).to_string()),
            None => JsonValue::Null.into(),
        },
        quaint::ValueType::Array(Some(ref items)) => JSArg::Array(
            items
                .iter()
                .map(value_to_js_arg)
                .collect::<serde_json::Result<Vec<JSArg>>>()?,
        ),
        quaint_value => JSArg::from(JsonValue::from(quaint_value.clone())),
    };
    Ok(res)
}
