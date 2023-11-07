use crate::conversion::JSArg;
use serde_json::value::Value as JsonValue;

pub fn value_to_js_arg(value: &quaint::Value) -> serde_json::Result<JSArg> {
    let res = match &value.typed {
        quaint::ValueType::Numeric(Some(bd)) => match bd.to_string().parse::<f64>() {
            Ok(double) => JSArg::from(JsonValue::from(double)),
            Err(_) => JSArg::from(JsonValue::from(value.clone())),
        },
        quaint::ValueType::Json(Some(s)) => JSArg::Value(s.to_owned()),
        quaint::ValueType::Bytes(Some(bytes)) => JSArg::Buffer(bytes.to_vec()),
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

// unit tests for value_to_js_arg
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
        let test_cases = vec![
           (
                // This is different than how mysql or postgres processes integral BigInt values.
                ValueType::Numeric(Some(1.into())),
                JSArg::Value(Value::Number("1.0".parse().unwrap()))
            ),
            (
                ValueType::Numeric(Some(BigDecimal::from_str("-1.1").unwrap())),
                JSArg::Value(Value::Number("-1.1".parse().unwrap())),
            ),
            (
                ValueType::Numeric(None),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::Json(Some(serde_json::json!({"a": 1}))),
                JSArg::Value(serde_json::json!({"a": 1})),
            ),
            (
                ValueType::Json(None),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::Date(Some(NaiveDate::from_ymd_opt(2020, 2, 29).unwrap())),
                JSArg::Value(Value::String("2020-02-29".to_string())),
            ),
            (
                ValueType::Date(None),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::DateTime(Some(Utc.with_ymd_and_hms(2020, 1, 1, 23, 13, 1).unwrap())),
                JSArg::Value(Value::String("2020-01-01T23:13:01+00:00".to_string())),
            ),
            (
                ValueType::DateTime(None),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::Time(Some(NaiveTime::from_hms_opt(23, 13, 1).unwrap())),
                JSArg::Value(Value::String("23:13:01".to_string())),
            ),
            (
                ValueType::Time(None),
                JSArg::Value(Value::Null)
            ),
            (
                ValueType::Array(Some(vec!(
                    ValueType::Numeric(Some(1.into())).into_value(),
                    ValueType::Numeric(None).into_value(),
                    ValueType::Time(Some(NaiveTime::from_hms_opt(23, 13, 1).unwrap())).into_value(),
                    ValueType::Time(None).into_value(),
                ))),
                JSArg::Array(vec!(
                    JSArg::Value(Value::Number("1.0".parse().unwrap())),
                    JSArg::Value(Value::Null),
                    JSArg::Value(Value::String("23:13:01".to_string())),
                    JSArg::Value(Value::Null),
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
