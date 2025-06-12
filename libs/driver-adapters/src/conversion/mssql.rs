use crate::conversion::JSArg;
use serde_json::value::Value as JsonValue;

#[rustfmt::skip]
pub fn value_to_js_arg(value: &quaint::Value) -> serde_json::Result<JSArg> {
    let res = match (&value.typed, value.native_column_type_name()) {
        (quaint::ValueType::DateTime(Some(dt)), _) => JSArg::Value(JsonValue::String(dt.naive_utc().to_string())),
        (quaint::ValueType::Json(Some(s)), _) => JSArg::Value(JsonValue::String(serde_json::to_string(s)?)),
        (quaint::ValueType::Bytes(Some(bytes)), _) => JSArg::Buffer(bytes.to_vec()),
        (quaint::ValueType::Int32(Some(value)), _) => JSArg::SafeInt(*value),
        (quaint::ValueType::Numeric(Some(bd)), _) =>  JSArg::Value(JsonValue::String(bd.to_string())),
        (quaint::ValueType::Array(Some(items)), _) => JSArg::Array(
            items
                .iter()
                .map(value_to_js_arg)
                .collect::<serde_json::Result<Vec<JSArg>>>()?,
        ),
        (quaint_value, _) => JSArg::from(JsonValue::from(quaint_value.clone())),
    };

    Ok(res)
}
