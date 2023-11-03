use crate::conversion::JSArg;
use serde_json::value::Value as JsonValue;

pub fn value_to_js_arg(value: &quaint::Value) -> serde_json::Result<JSArg> {
    let res = match &value.typed {
        quaint::ValueType::Numeric(bd) => match bd {
            // converting decimal to string to preserve the precision
            Some(bd) => match bd.to_string().parse::<f64>() {
                Ok(double) => JSArg::from(JsonValue::from(double)),
                Err(_) => JSArg::from(JsonValue::from(value.clone())),
            },
            None => JsonValue::Null.into(),
        },
        quaint::ValueType::Array(Some(ref items)) => JSArg::Array(values_to_js_args(items)?),
        _ => super::value_to_js_arg(value)?,
    };

    Ok(res)
}

pub fn values_to_js_args(values: &[quaint::Value<'_>]) -> serde_json::Result<Vec<JSArg>> {
    values.iter().map(value_to_js_arg).collect()
}
