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
        (quaint::ValueType::Numeric(bd), _) => match bd {
            // converting decimal to string to preserve the precision
            Some(bd) => JSArg::RawString(bd.to_string()),
            None => JsonValue::Null.into(),
        },
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
