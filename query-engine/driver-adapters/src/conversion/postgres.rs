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

#[cfg(test)]
mod test {
    use super::*;
    use bigdecimal::BigDecimal;
    use chrono::*;
    use quaint::ValueType;
    use serde_json::Value;
    use std::str::FromStr;

    #[test]
    #[rustfmt::skip]
    fn test_value_to_js_arg() {
        let test_cases: Vec<(quaint::Value, JSArg)> = vec![
            (
                ValueType::Numeric(Some(1.into())).into_value(),
                JSArg::RawString("1".to_string())
            ),
            (
                ValueType::Numeric(Some(BigDecimal::from_str("-1.1").unwrap())).into_value(),
                JSArg::RawString("-1.1".to_string()),
            ),
            (
                ValueType::Numeric(None).into_value(),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::Json(Some(serde_json::json!({"a": 1}))).into_value(),
                JSArg::RawString(r#"{"a":1}"#.to_string()),
            ),
            (
                ValueType::Json(None).into_value(),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::Date(Some(NaiveDate::from_ymd_opt(2020, 2, 29).unwrap())).into_value(),
                JSArg::Value(Value::String("2020-02-29".to_string())),
            ),
            (
                ValueType::Date(None).into_value(),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::DateTime(Some(Utc.with_ymd_and_hms(2020, 1, 1, 23, 13, 1).unwrap())).into_value().with_native_column_type(Some("DATE")),
                JSArg::RawString("2020-01-01".to_string()),
            ),
            (
                ValueType::DateTime(Some(Utc.with_ymd_and_hms(2020, 1, 1, 23, 13, 1).unwrap())).into_value().with_native_column_type(Some("TIME")),
                JSArg::RawString("23:13:01".to_string()),
            ),
            (
                ValueType::DateTime(Some(Utc.with_ymd_and_hms(2020, 1, 1, 23, 13, 1).unwrap())).into_value().with_native_column_type(Some("TIMETZ")),
                JSArg::RawString("23:13:01".to_string()),
            ),
            (
                ValueType::DateTime(None).into_value(),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::Time(Some(NaiveTime::from_hms_opt(23, 13, 1).unwrap())).into_value(),
                JSArg::Value(Value::String("23:13:01".to_string())),
            ),
            (
                ValueType::Time(None).into_value(),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::Array(Some(vec!(
                    ValueType::Numeric(Some(1.into())).into_value(),
                    ValueType::Numeric(None).into_value(),
                    ValueType::Time(Some(NaiveTime::from_hms_opt(23, 13, 1).unwrap())).into_value(),
                    ValueType::Time(None).into_value(),
                ))).into_value(),
                JSArg::Array(vec!(
                    JSArg::RawString("1".to_string()),
                    JSArg::Value(Value::Null),
                    JSArg::Value(Value::String("23:13:01".to_string())),
                    JSArg::Value(Value::Null),
                ))
            ),
        ];

        let mut errors: Vec<String> = vec![];
        for (val, expected) in test_cases {
            let actual = value_to_js_arg(&val).unwrap();
            if actual != expected {
                errors.push(format!("transforming: {:?}, expected: {:?}, actual: {:?}", &val, expected, actual));
            }
        }
        assert_eq!(errors.len(), 0, "{}", errors.join("\n"));
    }
}
