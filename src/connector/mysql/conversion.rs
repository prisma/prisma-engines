use crate::{
    ast::ParameterizedValue,
    connector::queryable::{ToColumnNames, ToRow},
};
#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use mysql as my;

pub fn conv_params<'a>(params: &[ParameterizedValue<'a>]) -> my::Params {
    if params.is_empty() {
        // If we don't use explicit 'Empty',
        // mysql crashes with 'internal error: entered unreachable code'
        my::Params::Empty
    } else {
        my::Params::Positional(params.iter().map(|x| x.into()).collect::<Vec<my::Value>>())
    }
}

impl ToRow for my::Row {
    fn to_result_row<'b>(&'b self) -> crate::Result<Vec<ParameterizedValue<'static>>> {
        fn convert(row: &my::Row, i: usize) -> crate::Result<ParameterizedValue<'static>> {
            // TODO: It would prob. be better to inver via Column::column_type()
            let raw_value = row.as_ref(i).unwrap_or(&my::Value::NULL);
            let res = match raw_value {
                my::Value::NULL => ParameterizedValue::Null,
                my::Value::Bytes(b) => {
                    ParameterizedValue::Text(String::from_utf8(b.to_vec())?.into())
                }
                my::Value::Int(i) => ParameterizedValue::Integer(*i),
                // TOOD: This is unsafe
                my::Value::UInt(i) => ParameterizedValue::Integer(*i as i64),
                my::Value::Float(f) => ParameterizedValue::Real(*f),
                #[cfg(feature = "chrono-0_4")]
                my::Value::Date(..) => {
                    let ts: NaiveDateTime = row.get(i).unwrap();
                    ParameterizedValue::DateTime(DateTime::<Utc>::from_utc(ts, Utc))
                }
                #[cfg(feature = "chrono-0_4")]
                my::Value::Time(is_neg, days, hours, minutes, seconds, micros) => {
                    let days = Duration::days(i64::from(*days));
                    let hours = Duration::hours(i64::from(*hours));
                    let minutes = Duration::minutes(i64::from(*minutes));
                    let seconds = Duration::seconds(i64::from(*seconds));
                    let micros = Duration::microseconds(i64::from(*micros));

                    let time = days
                        .checked_add(&hours)
                        .and_then(|t| t.checked_add(&minutes))
                        .and_then(|t| t.checked_add(&seconds))
                        .and_then(|t| t.checked_add(&micros))
                        .unwrap();

                    let duration = time.to_std().unwrap();
                    let f_time =
                        duration.as_secs() as f64 + f64::from(duration.subsec_micros()) * 1e-6;

                    ParameterizedValue::Real(if *is_neg { -f_time } else { f_time })
                }
                #[cfg(not(feature = "chrono-0_4"))]
                typ => panic!(
                    "Value of type {:?} is not supported with the current configuration",
                    typ
                ),
            };

            Ok(res)
        }

        let mut row = Vec::new();

        for i in 0..self.len() {
            row.push(convert(self, i)?);
        }

        Ok(row)
    }
}

impl<'a> ToColumnNames for my::Stmt<'a> {
    fn to_column_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        if let Some(columns) = self.columns_ref() {
            for column in columns {
                names.push(String::from(column.name_str()));
            }
        }

        names
    }
}
