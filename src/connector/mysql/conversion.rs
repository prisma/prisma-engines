use crate::{ast::ParameterizedValue, connector::queryable::TakeRow};
#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use mysql_async as my;
use mysql_async::Value as MyValue;
use rust_decimal::prelude::ToPrimitive;

pub fn conv_params<'a>(params: &[ParameterizedValue<'a>]) -> my::Params {
    if params.is_empty() {
        // If we don't use explicit 'Empty',
        // mysql crashes with 'internal error: entered unreachable code'
        my::Params::Empty
    } else {
        my::Params::Positional(params.iter().map(|x| x.into()).collect::<Vec<my::Value>>())
    }
}

impl TakeRow for my::Row {
    fn take_result_row<'b>(&'b mut self) -> crate::Result<Vec<ParameterizedValue<'static>>> {
        fn convert(row: &mut my::Row, i: usize) -> crate::Result<ParameterizedValue<'static>> {
            let res = match row.take(i).unwrap_or(my::Value::NULL) {
                my::Value::NULL => ParameterizedValue::Null,
                my::Value::Bytes(b) => ParameterizedValue::Text(String::from_utf8(b.to_vec())?.into()),
                my::Value::Int(i) => ParameterizedValue::Integer(i),
                // TOOD: This is unsafe
                my::Value::UInt(i) => ParameterizedValue::Integer(i as i64),
                my::Value::Float(f) => ParameterizedValue::from(f),
                #[cfg(feature = "chrono-0_4")]
                my::Value::Date(year, month, day, hour, min, sec, micro) => {
                    let time = NaiveTime::from_hms_micro(hour as u32, min as u32, sec as u32, micro);

                    let date = NaiveDate::from_ymd(year as i32, month as u32, day as u32);
                    let dt = NaiveDateTime::new(date, time);

                    ParameterizedValue::DateTime(DateTime::<Utc>::from_utc(dt, Utc))
                }
                #[cfg(feature = "chrono-0_4")]
                my::Value::Time(is_neg, days, hours, minutes, seconds, micros) => {
                    let days = Duration::days(i64::from(days));
                    let hours = Duration::hours(i64::from(hours));
                    let minutes = Duration::minutes(i64::from(minutes));
                    let seconds = Duration::seconds(i64::from(seconds));
                    let micros = Duration::microseconds(i64::from(micros));

                    let time = days
                        .checked_add(&hours)
                        .and_then(|t| t.checked_add(&minutes))
                        .and_then(|t| t.checked_add(&seconds))
                        .and_then(|t| t.checked_add(&micros))
                        .unwrap();

                    let duration = time.to_std().unwrap();
                    let f_time = duration.as_secs() as f64 + f64::from(duration.subsec_micros()) * 1e-6;
                    let f_time = if is_neg { -f_time } else { f_time };

                    ParameterizedValue::from(f_time)
                }
                #[cfg(not(feature = "chrono-0_4"))]
                typ => panic!(
                    "Value of type {:?} is not supported with the current configuration",
                    typ
                ),
            };

            Ok(res)
        }

        let mut row = Vec::with_capacity(self.len());

        for i in 0..self.len() {
            row.push(convert(self, i)?);
        }

        Ok(row)
    }
}

impl<'a> From<ParameterizedValue<'a>> for MyValue {
    fn from(pv: ParameterizedValue<'a>) -> MyValue {
        match pv {
            ParameterizedValue::Null => MyValue::NULL,
            ParameterizedValue::Integer(i) => MyValue::Int(i),
            ParameterizedValue::Real(f) => MyValue::Float(f.to_f64().expect("Decimal is not a f64.")),
            ParameterizedValue::Text(s) => MyValue::Bytes((&*s).as_bytes().to_vec()),
            ParameterizedValue::Enum(s) => MyValue::Bytes((&*s).as_bytes().to_vec()),
            ParameterizedValue::Boolean(b) => MyValue::Int(b as i64),
            ParameterizedValue::Char(c) => MyValue::Bytes(vec![c as u8]),
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(json) => {
                let s = serde_json::to_string(&json).expect("Cannot convert JSON to String.");

                MyValue::Bytes(s.into_bytes())
            }
            #[cfg(feature = "array")]
            ParameterizedValue::Array(_) => unimplemented!("Arrays are not supported for mysql."),
            #[cfg(feature = "uuid-0_8")]
            ParameterizedValue::Uuid(u) => MyValue::Bytes(u.to_hyphenated().to_string().into_bytes()),
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(dt) => MyValue::Date(
                dt.year() as u16,
                dt.month() as u8,
                dt.day() as u8,
                dt.hour() as u8,
                dt.minute() as u8,
                dt.second() as u8,
                dt.timestamp_subsec_micros(),
            ),
        }
    }
}
