use std::borrow::Cow;
use std::str::FromStr;

pub use crate::types::{ColumnType, JSResultSet};
use quaint::bigdecimal::BigDecimal;
use quaint::chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use quaint::{
    connector::ResultSet as QuaintResultSet,
    error::{Error as QuaintError, ErrorKind},
    Value as QuaintValue,
};

impl TryFrom<JSResultSet> for QuaintResultSet {
    type Error = quaint::error::Error;

    fn try_from(js_result_set: JSResultSet) -> Result<Self, Self::Error> {
        let JSResultSet {
            rows,
            column_names,
            column_types,
            last_insert_id,
        } = js_result_set;

        let mut quaint_rows = Vec::with_capacity(rows.len());

        for row in rows {
            let mut quaint_row = Vec::with_capacity(column_types.len());

            for (i, row) in row.into_iter().enumerate() {
                let column_type = column_types[i];
                let column_name = column_names[i].as_str();

                quaint_row.push(js_value_to_quaint(row, column_type, column_name)?);
            }

            quaint_rows.push(quaint_row);
        }

        let last_insert_id = last_insert_id.and_then(|id| id.parse::<u64>().ok());
        let mut quaint_result_set = QuaintResultSet::new(column_names, quaint_rows);

        // Not a fan of this (extracting the `Some` value from an `Option` and pass it to a method that creates a new `Some` value),
        // but that's Quaint's ResultSet API and that's how the MySQL connector does it.
        // Sqlite, on the other hand, uses a `last_insert_id.unwrap_or(0)` approach.
        if let Some(last_insert_id) = last_insert_id {
            quaint_result_set.set_last_insert_id(last_insert_id);
        }

        Ok(quaint_result_set)
    }
}

fn conversion_error(args: &std::fmt::Arguments) -> QuaintError {
    let msg = match args.as_str() {
        Some(s) => Cow::Borrowed(s),
        None => Cow::Owned(args.to_string()),
    };
    QuaintError::builder(ErrorKind::ConversionError(msg)).build()
}

macro_rules! conversion_error {
  ($($arg:tt)*) => {
      conversion_error(&format_args!($($arg)*))
  };
}

/// Handle data-type conversion from a JSON value to a Quaint value.
/// This is used for most data types, except those that require connector-specific handling, e.g., `ColumnType::Boolean`.
pub fn js_value_to_quaint(
    json_value: serde_json::Value,
    column_type: ColumnType,
    column_name: &str,
) -> quaint::Result<QuaintValue<'static>> {
    let parse_number_as_i64 = |n: &serde_json::Number| {
        n.as_i64().ok_or(conversion_error!(
            "number must be an integer in column '{column_name}', got '{n}'"
        ))
    };

    //  Note for the future: it may be worth revisiting how much bloat so many panics with different static
    // strings add to the compiled artefact, and in case we should come up with a restricted set of panic
    // messages, or even find a way of removing them altogether.
    match column_type {
        ColumnType::Int32 => match json_value {
            serde_json::Value::Number(n) => {
                // n.as_i32() is not implemented, so we need to downcast from i64 instead
                parse_number_as_i64(&n)
                    .and_then(|n| -> quaint::Result<i32> {
                        n.try_into()
                            .map_err(|e| conversion_error!("cannot convert {n} to i32 in column '{column_name}': {e}"))
                    })
                    .map(QuaintValue::int32)
            }
            serde_json::Value::String(s) => s.parse::<i32>().map(QuaintValue::int32).map_err(|e| {
                conversion_error!("string-encoded number must be an i32 in column '{column_name}', got {s}: {e}")
            }),
            serde_json::Value::Null => Ok(QuaintValue::null_int32()),
            mismatch => Err(conversion_error!(
                "expected an i32 number in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Int64 => match json_value {
            serde_json::Value::Number(n) => parse_number_as_i64(&n).map(QuaintValue::int64),
            serde_json::Value::String(s) => s.parse::<i64>().map(QuaintValue::int64).map_err(|e| {
                conversion_error!("string-encoded number must be an i64 in column '{column_name}', got {s}: {e}")
            }),
            serde_json::Value::Null => Ok(QuaintValue::null_int64()),
            mismatch => Err(conversion_error!(
                "expected a string or number in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Float => match json_value {
            // n.as_f32() is not implemented, so we need to downcast from f64 instead.
            // We assume that the JSON value is a valid f32 number, but we check for overflows anyway.
            serde_json::Value::Number(n) => n
                .as_f64()
                .ok_or(conversion_error!(
                    "number must be a float in column '{column_name}', got {n}"
                ))
                .and_then(f64_to_f32)
                .map(QuaintValue::float),
            serde_json::Value::Null => Ok(QuaintValue::null_float()),
            mismatch => Err(conversion_error!(
                "expected an f32 number in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Double => match json_value {
            serde_json::Value::Number(n) => n.as_f64().map(QuaintValue::double).ok_or(conversion_error!(
                "number must be a f64 in column '{column_name}', got {n}"
            )),
            serde_json::Value::Null => Ok(QuaintValue::null_double()),
            mismatch => Err(conversion_error!(
                "expected an f64 number in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Numeric => match json_value {
            serde_json::Value::String(s) => BigDecimal::from_str(&s).map(QuaintValue::numeric).map_err(|e| {
                conversion_error!("invalid numeric value when parsing {s} in column '{column_name}': {e}")
            }),
            serde_json::Value::Number(n) => BigDecimal::from_str(&n.to_string())
                .map_err(|_| conversion_error!("number must be an f64 in column '{column_name}', got {n}"))
                .map(QuaintValue::numeric),
            serde_json::Value::Null => Ok(QuaintValue::null_numeric()),
            mismatch => Err(conversion_error!(
                "expected a string-encoded number in column '{column_name}', found {mismatch}",
            )),
        },
        ColumnType::Boolean => match json_value {
            serde_json::Value::Bool(b) => Ok(QuaintValue::boolean(b)),
            serde_json::Value::Null => Ok(QuaintValue::null_boolean()),
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(0) => Ok(QuaintValue::boolean(false)),
                Some(1) => Ok(QuaintValue::boolean(true)),
                _ => Err(conversion_error!(
                    "expected number-encoded boolean to be 0 or 1 in column '{column_name}', got {n}"
                )),
            },
            serde_json::Value::String(s) => match s.as_str() {
                "false" | "FALSE" | "0" => Ok(QuaintValue::boolean(false)),
                "true" | "TRUE" | "1" => Ok(QuaintValue::boolean(true)),
                _ => Err(conversion_error!(
                    "expected string-encoded boolean in column '{column_name}', got {s}"
                )),
            },
            mismatch => Err(conversion_error!(
                "expected a boolean in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Character => match json_value {
            serde_json::Value::String(s) => match s.chars().next() {
                Some(c) => Ok(QuaintValue::character(c)),
                None => Ok(QuaintValue::null_character()),
            },
            serde_json::Value::Null => Ok(QuaintValue::null_character()),
            mismatch => Err(conversion_error!(
                "expected a string in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Text => match json_value {
            serde_json::Value::String(s) => Ok(QuaintValue::text(s)),
            serde_json::Value::Null => Ok(QuaintValue::null_text()),
            mismatch => Err(conversion_error!(
                "expected a string in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Date => match json_value {
            serde_json::Value::String(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(QuaintValue::date)
                .map_err(|_| conversion_error!("expected a date string in column '{column_name}', got {s}")),
            serde_json::Value::Null => Ok(QuaintValue::null_date()),
            mismatch => Err(conversion_error!(
                "expected a string in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Time => match json_value {
            serde_json::Value::String(s) => NaiveTime::parse_from_str(&s, "%H:%M:%S%.f")
                .map(QuaintValue::time)
                .map_err(|_| conversion_error!("expected a time string in column '{column_name}', got {s}")),
            serde_json::Value::Null => Ok(QuaintValue::null_time()),
            mismatch => Err(conversion_error!(
                "expected a string in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::DateTime => match json_value {
            // TODO: change parsing order to prefer RFC3339
            serde_json::Value::String(s) => quaint::chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f")
                .map(|dt| DateTime::from_utc(dt, Utc))
                .or_else(|_| DateTime::parse_from_rfc3339(&s).map(DateTime::<Utc>::from))
                .map(QuaintValue::datetime)
                .map_err(|_| conversion_error!("expected a datetime string in column '{column_name}', found {s}")),
            serde_json::Value::Null => Ok(QuaintValue::null_datetime()),
            mismatch => Err(conversion_error!(
                "expected a string in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Json => {
            match json_value {
                // DbNull
                serde_json::Value::Null => Ok(QuaintValue::null_json()),
                serde_json::Value::String(s) => serde_json::from_str(&s)
                    .map_err(|_| conversion_error!("Failed to parse incoming json from a driver adapter"))
                    .map(QuaintValue::json),
                json => Ok(QuaintValue::json(json)),
            }
        }
        ColumnType::Enum => match json_value {
            serde_json::Value::String(s) => Ok(QuaintValue::enum_variant(s)),
            serde_json::Value::Null => Ok(QuaintValue::null_enum()),
            mismatch => Err(conversion_error!(
                "expected a string in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::Bytes => match json_value {
            serde_json::Value::String(s) => Ok(QuaintValue::bytes(s.into_bytes())),
            serde_json::Value::Array(array) => array
                .iter()
                .map(|value| value.as_i64().and_then(|maybe_byte| maybe_byte.try_into().ok()))
                .collect::<Option<Cow<[u8]>>>()
                .map(QuaintValue::bytes)
                .ok_or(conversion_error!(
                    "elements of the array in column '{column_name}' must be u8"
                )),
            serde_json::Value::Null => Ok(QuaintValue::null_bytes()),
            mismatch => Err(conversion_error!(
                "expected a string or an array in column '{column_name}', found {mismatch}",
            )),
        },
        ColumnType::Uuid => match json_value {
            serde_json::Value::String(s) => uuid::Uuid::parse_str(&s)
                .map(QuaintValue::uuid)
                .map_err(|_| conversion_error!("Expected a UUID string in column '{column_name}'")),
            serde_json::Value::Null => Ok(QuaintValue::null_bytes()),
            mismatch => Err(conversion_error!(
                "Expected a UUID string in column '{column_name}', found {mismatch}"
            )),
        },
        ColumnType::UnknownNumber => match json_value {
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(QuaintValue::int64)
                .or(n.as_f64().map(QuaintValue::double))
                .ok_or(conversion_error!(
                    "number must be an i64 or f64 in column '{column_name}', got {n}"
                )),
            mismatch => Err(conversion_error!(
                "expected a either an i64 or a f64 in column '{column_name}', found {mismatch}",
            )),
        },

        ColumnType::Int32Array => js_array_to_quaint(ColumnType::Int32, json_value, column_name),
        ColumnType::Int64Array => js_array_to_quaint(ColumnType::Int64, json_value, column_name),
        ColumnType::FloatArray => js_array_to_quaint(ColumnType::Float, json_value, column_name),
        ColumnType::DoubleArray => js_array_to_quaint(ColumnType::Double, json_value, column_name),
        ColumnType::NumericArray => js_array_to_quaint(ColumnType::Numeric, json_value, column_name),
        ColumnType::BooleanArray => js_array_to_quaint(ColumnType::Boolean, json_value, column_name),
        ColumnType::CharacterArray => js_array_to_quaint(ColumnType::Character, json_value, column_name),
        ColumnType::TextArray => js_array_to_quaint(ColumnType::Text, json_value, column_name),
        ColumnType::DateArray => js_array_to_quaint(ColumnType::Date, json_value, column_name),
        ColumnType::TimeArray => js_array_to_quaint(ColumnType::Time, json_value, column_name),
        ColumnType::DateTimeArray => js_array_to_quaint(ColumnType::DateTime, json_value, column_name),
        ColumnType::JsonArray => js_array_to_quaint(ColumnType::Json, json_value, column_name),
        ColumnType::EnumArray => js_array_to_quaint(ColumnType::Enum, json_value, column_name),
        ColumnType::BytesArray => js_array_to_quaint(ColumnType::Bytes, json_value, column_name),
        ColumnType::UuidArray => js_array_to_quaint(ColumnType::Uuid, json_value, column_name),

        unimplemented => {
            todo!("support column type {:?} in column {}", unimplemented, column_name)
        }
    }
}

fn js_array_to_quaint(
    base_type: ColumnType,
    json_value: serde_json::Value,
    column_name: &str,
) -> quaint::Result<QuaintValue<'static>> {
    match json_value {
        serde_json::Value::Array(array) => Ok(QuaintValue::array(
            array
                .into_iter()
                .enumerate()
                .map(|(index, elem)| js_value_to_quaint(elem, base_type, &format!("{column_name}[{index}]")))
                .collect::<quaint::Result<Vec<_>>>()?,
        )),
        serde_json::Value::Null => Ok(QuaintValue::null_array()),
        mismatch => Err(conversion_error!(
            "expected an array in column '{column_name}', found {mismatch}",
        )),
    }
}

/// Coerce a `f64` to a `f32`, asserting that the conversion is lossless.
/// Note that, when overflow occurs during conversion, the result is `infinity`.
fn f64_to_f32(x: f64) -> quaint::Result<f32> {
    let y = x as f32;

    if x.is_finite() == y.is_finite() {
        Ok(y)
    } else {
        Err(conversion_error!("f32 overflow during conversion"))
    }
}

#[cfg(test)]
mod proxy_test {
    use quaint::bigdecimal::num_bigint::BigInt;
    use serde_json::json;

    use super::*;

    #[track_caller]
    fn test_null<'a, T: Into<QuaintValue<'a>>>(quaint_none: T, column_type: ColumnType) {
        let json_value = serde_json::Value::Null;
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, quaint_none.into());
    }

    #[test]
    fn js_value_binary_to_quaint() {
        let column_type = ColumnType::Bytes;

        // null
        test_null(QuaintValue::null_bytes(), column_type);

        // ""
        let json_value = serde_json::Value::String("".to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::bytes(vec![]));

        // "hello"
        let json_value = serde_json::Value::String("hello".to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::bytes(vec![104, 101, 108, 108, 111]));
    }

    #[test]
    fn js_value_int32_to_quaint() {
        let column_type = ColumnType::Int32;

        // null
        test_null(QuaintValue::null_int32(), column_type);

        // 0
        let n: i32 = 0;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::int32(n));

        // max
        let n: i32 = i32::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::int32(n));

        // min
        let n: i32 = i32::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::int32(n));

        // string-encoded
        let n = i32::MAX;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::int32(n));
    }

    #[test]
    fn js_value_int64_to_quaint() {
        let column_type = ColumnType::Int64;

        // null
        test_null(QuaintValue::null_int64(), column_type);

        // 0
        let n: i64 = 0;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::int64(n));

        // max
        let n: i64 = i64::MAX;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::int64(n));

        // min
        let n: i64 = i64::MIN;
        let json_value = serde_json::Value::String(n.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::int64(n));

        // number-encoded
        let n: i64 = (1 << 53) - 1; // max JS safe integer
        let json_value = serde_json::Value::Number(serde_json::Number::from(n));
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::int64(n));
    }

    #[test]
    fn js_value_float_to_quaint() {
        let column_type = ColumnType::Float;

        // null
        test_null(QuaintValue::null_float(), column_type);

        // 0
        let n: f32 = 0.0;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::float(n));

        // max
        let n: f32 = f32::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::float(n));

        // min
        let n: f32 = f32::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n.into()).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::float(n));
    }

    #[test]
    fn js_value_double_to_quaint() {
        let column_type = ColumnType::Double;

        // null
        test_null(QuaintValue::null_double(), column_type);

        // 0
        let n: f64 = 0.0;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::double(n));

        // max
        let n: f64 = f64::MAX;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::double(n));

        // min
        let n: f64 = f64::MIN;
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::double(n));
    }

    #[test]
    fn js_value_numeric_to_quaint() {
        let column_type = ColumnType::Numeric;

        // null
        test_null(QuaintValue::null_numeric(), column_type);

        let n_as_string = "1234.99";
        let decimal = BigDecimal::new(BigInt::parse_bytes(b"123499", 10).unwrap(), 2);

        let json_value = serde_json::Value::String(n_as_string.into());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::numeric(decimal));

        let n_as_string = "1234.999999";
        let decimal = BigDecimal::new(BigInt::parse_bytes(b"1234999999", 10).unwrap(), 6);

        let json_value = serde_json::Value::String(n_as_string.into());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::numeric(decimal));
    }

    #[test]
    fn js_value_boolean_to_quaint() {
        let column_type = ColumnType::Boolean;

        // null
        test_null(QuaintValue::null_boolean(), column_type);

        // true
        for truthy_value in [json!(true), json!(1), json!("true"), json!("TRUE"), json!("1")] {
            let quaint_value = js_value_to_quaint(truthy_value, column_type, "column_name").unwrap();
            assert_eq!(quaint_value, QuaintValue::boolean(true));
        }

        // false
        for falsy_value in [json!(false), json!(0), json!("false"), json!("FALSE"), json!("0")] {
            let quaint_value = js_value_to_quaint(falsy_value, column_type, "column_name").unwrap();
            assert_eq!(quaint_value, QuaintValue::boolean(false));
        }
    }

    #[test]
    fn js_value_char_to_quaint() {
        let column_type = ColumnType::Character;

        // null
        test_null(QuaintValue::null_character(), column_type);

        let c = 'c';
        let json_value = serde_json::Value::String(c.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::character(c));
    }

    #[test]
    fn js_value_text_to_quaint() {
        let column_type = ColumnType::Text;

        // null
        test_null(QuaintValue::null_text(), column_type);

        let s = "some text";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::text(s));
    }

    #[test]
    fn js_value_date_to_quaint() {
        let column_type = ColumnType::Date;

        // null
        test_null(QuaintValue::null_date(), column_type);

        let s = "2023-01-01";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        assert_eq!(quaint_value, QuaintValue::date(date));
    }

    #[test]
    fn js_value_time_to_quaint() {
        let column_type = ColumnType::Time;

        // null
        test_null(QuaintValue::null_time(), column_type);

        let s = "23:59:59";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        let time: NaiveTime = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
        assert_eq!(quaint_value, QuaintValue::time(time));

        let s = "13:02:20.321";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        let time: NaiveTime = NaiveTime::from_hms_milli_opt(13, 2, 20, 321).unwrap();
        assert_eq!(quaint_value, QuaintValue::time(time));
    }

    #[test]
    fn js_value_datetime_to_quaint() {
        let column_type = ColumnType::DateTime;

        // null
        test_null(QuaintValue::null_datetime(), column_type);

        let s = "2023-01-01 23:59:59.415";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_milli_opt(23, 59, 59, 415)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value, QuaintValue::datetime(datetime));

        let s = "2023-01-01 23:59:59.123456";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_micro_opt(23, 59, 59, 123_456)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value, QuaintValue::datetime(datetime));

        let s = "2023-01-01 23:59:59";
        let json_value = serde_json::Value::String(s.to_string());
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        let datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_milli_opt(23, 59, 59, 0)
            .unwrap();
        let datetime = DateTime::from_utc(datetime, Utc);
        assert_eq!(quaint_value, QuaintValue::datetime(datetime));
    }

    #[test]
    fn js_value_json_to_quaint() {
        let column_type = ColumnType::Json;

        // null
        test_null(QuaintValue::null_json(), column_type);

        let json = json!({
            "key": "value",
            "nested": [
                true,
                false,
                1,
                null
            ]
        });
        let json_value = json.clone();
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::json(json.clone()));
    }

    #[test]
    fn js_value_enum_to_quaint() {
        let column_type = ColumnType::Enum;

        // null
        test_null(QuaintValue::null_enum(), column_type);

        let s = "some enum variant";
        let json_value = serde_json::Value::String(s.to_string());

        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();
        assert_eq!(quaint_value, QuaintValue::enum_variant(s));
    }

    #[test]
    fn js_int32_array_to_quaint() {
        let column_type = ColumnType::Int32Array;
        test_null(QuaintValue::null_array(), column_type);

        let json_value = json!([1, 2, 3]);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        assert_eq!(
            quaint_value,
            QuaintValue::array(vec![
                QuaintValue::int32(1),
                QuaintValue::int32(2),
                QuaintValue::int32(3)
            ])
        );

        let json_value = json!([1, 2, {}]);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        assert_eq!(
            quaint_value.err().unwrap().to_string(),
            "Conversion failed: expected an i32 number in column 'column_name[2]', found {}"
        );
    }

    #[test]
    fn js_text_array_to_quaint() {
        let column_type = ColumnType::TextArray;
        test_null(QuaintValue::null_array(), column_type);

        let json_value = json!(["hi", "there"]);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name").unwrap();

        assert_eq!(
            quaint_value,
            QuaintValue::array(vec![QuaintValue::text("hi"), QuaintValue::text("there"),])
        );

        let json_value = json!([10]);
        let quaint_value = js_value_to_quaint(json_value, column_type, "column_name");

        assert_eq!(
            quaint_value.err().unwrap().to_string(),
            "Conversion failed: expected a string in column 'column_name[0]', found 10"
        );
    }
}
