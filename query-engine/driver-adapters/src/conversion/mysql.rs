use crate::conversion::JSArg;
use serde_json::value::Value as JsonValue;

const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.f";
const DATE_FORMAT: &str = "%Y-%m-%d";
const TIME_FORMAT: &str = "%H:%M:%S%.f";

#[rustfmt::skip]
pub fn value_to_js_arg(value: &quaint::Value) -> serde_json::Result<JSArg> {
    let res = match &value.typed {
        quaint::ValueType::Numeric(Some(bd)) => JSArg::Value(JsonValue::String(bd.to_string())),
        quaint::ValueType::Json(Some(s)) => JSArg::Value(JsonValue::String(serde_json::to_string(s)?)),
        quaint::ValueType::Bytes(Some(bytes)) => JSArg::Buffer(bytes.to_vec()),
        quaint::ValueType::Date(Some(d)) => JSArg::Value(JsonValue::String(d.format(DATE_FORMAT).to_string())),
        quaint::ValueType::DateTime(Some(dt)) => JSArg::Value(JsonValue::String(dt.format(DATETIME_FORMAT).to_string())),
        quaint::ValueType::Time(Some(t)) => JSArg::Value(JsonValue::String(t.format(TIME_FORMAT).to_string())),
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

#[cfg(test)]
mod test {
    use super::*;
    use bigdecimal::BigDecimal;
    use chrono::*;
    use quaint::ValueType;
    use std::str::FromStr;

    #[test]
    #[rustfmt::skip]
    fn test_value_to_js_arg() {
            let test_cases = vec![
            (
                ValueType::Numeric(Some(1.into())), 
                JSArg::Value(JsonValue::String("1".to_string()))
            ),
            (
                ValueType::Numeric(Some(BigDecimal::from_str("-1.1").unwrap())),
                JSArg::Value(JsonValue::String("-1.1".to_string()))
            ),
            (
                ValueType::Numeric(None),
                JSArg::Value(JsonValue::Null)
            ),
            (
                ValueType::Json(Some(serde_json::json!({"a": 1}))),
                JSArg::Value(JsonValue::String("{\"a\":1}".to_string()))
            ),
            (
                ValueType::Json(None),
                JSArg::Value(JsonValue::Null)
            ),
            (
                ValueType::Date(Some(NaiveDate::from_ymd_opt(2020, 2, 29).unwrap())),
                JSArg::Value(JsonValue::String("2020-02-29".to_string()))
            ),
            (
                ValueType::Date(None),
                JSArg::Value(JsonValue::Null)
            ),
            (
                ValueType::DateTime(Some(Utc.with_ymd_and_hms(2020, 1, 1, 23, 13, 1).unwrap().with_nanosecond(100).unwrap())),
                JSArg::Value(JsonValue::String("2020-01-01 23:13:01.000000100".to_string()))
            ),
            (
                ValueType::DateTime(None),
                JSArg::Value(JsonValue::Null)
            ),
            (
                ValueType::Time(Some(NaiveTime::from_hms_opt(23, 13, 1).unwrap().with_nanosecond(1200).unwrap())),
                JSArg::Value(JsonValue::String("23:13:01.000001200".to_string()))
            ),
            (
                ValueType::Time(None),
                JSArg::Value(JsonValue::Null)
            ),
            (
                ValueType::Array(Some(vec!(
                    ValueType::Numeric(Some(1.into())).into_value(),
                    ValueType::Numeric(None).into_value(),
                    ValueType::Time(Some(NaiveTime::from_hms_opt(23, 13, 1).unwrap())).into_value(),
                ))),
                JSArg::Array(vec!(
                    JSArg::Value(JsonValue::String("1".to_string())),
                    JSArg::Value(JsonValue::Null),
                    JSArg::Value(JsonValue::String("23:13:01".to_string()))
                ))
            ),
        ];

        let mut errors: Vec<String> = vec![];
        for (val, expected) in test_cases {
            let actual = value_to_js_arg(&val.clone().into_value()).unwrap();
            if actual != expected {
                errors.push(format!("transforming: {:?}, expected: {:?}, actual: {:?}", &val, expected, actual));
            }
        }
        assert_eq!(errors.len(), 0, "{}", errors.join("\n"));
    }
}
