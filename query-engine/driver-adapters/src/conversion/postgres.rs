use crate::conversion::JSArg;
use chrono::format::StrftimeItems;
use once_cell::sync::Lazy;
use serde_json::value::Value as JsonValue;

static TIME_FMT: Lazy<StrftimeItems> = Lazy::new(|| StrftimeItems::new("%H:%M:%S%.f"));

pub fn value_to_js_arg(value: &quaint::Value) -> serde_json::Result<JSArg> {
    let res = match (&value.typed, value.native_column_type_name()) {
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
        (quaint::ValueType::Array(Some(items)), _) => JSArg::Array(values_to_js_args(items)?),
        _ => super::value_to_js_arg(value)?,
    };

    Ok(res)
}

pub fn values_to_js_args(values: &[quaint::Value<'_>]) -> serde_json::Result<Vec<JSArg>> {
    values.iter().map(value_to_js_arg).collect()
}
