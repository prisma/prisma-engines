use crate::conversion::JSArg;
use chrono::format::StrftimeItems;
use once_cell::sync::Lazy;
use serde_json::value::Value as JsonValue;

static TIME_FMT: Lazy<StrftimeItems> = Lazy::new(|| StrftimeItems::new("%H:%M:%S%.f"));

pub fn values_to_js_args(values: &[quaint::Value<'_>]) -> serde_json::Result<Vec<JSArg>> {
    let mut args = Vec::with_capacity(values.len());

    for qv in values {
        let res = match (&qv.typed, qv.native_column_type_name()) {
            (quaint::ValueType::DateTime(value), Some("DATE")) => match value {
                Some(value) => JSArg::RawString(value.date_naive().to_string()),
                None => JsonValue::Null.into(),
            },
            (quaint::ValueType::DateTime(value), Some("TIME")) => match value {
                Some(value) => JSArg::RawString(value.time().to_string()),
                None => JsonValue::Null.into(),
            },
            (quaint::ValueType::DateTime(value), Some("TIMETZ")) => match value {
                Some(value) => JSArg::RawString(value.time().format_with_items(TIME_FMT.clone()).to_string()),
                None => JsonValue::Null.into(),
            },
            (quaint::ValueType::DateTime(value), _) => match value {
                Some(value) => JSArg::RawString(value.naive_utc().to_string()),
                None => JsonValue::Null.into(),
            },
            (quaint::ValueType::Json(s), _) => match s {
                Some(ref s) => {
                    let json_str = serde_json::to_string(s)?;
                    JSArg::RawString(json_str)
                }
                None => JsonValue::Null.into(),
            },
            (quaint::ValueType::Bytes(bytes), _) => match bytes {
                Some(bytes) => JSArg::Buffer(bytes.to_vec()),
                None => JsonValue::Null.into(),
            },
            (quaint::ValueType::Array(Some(items)), _) => JSArg::Array(values_to_js_args(items)?),
            (quaint_value, _) => JSArg::from(JsonValue::from(quaint_value.clone())),
        };

        args.push(res);
    }

    Ok(args)
}
