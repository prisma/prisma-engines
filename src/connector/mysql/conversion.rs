use crate::{
    ast::ParameterizedValue,
    connector::queryable::{ToColumnNames, ToRow},
};
#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, Duration, NaiveDate, Utc};
use mysql as my;

pub fn conv_params<'a>(params: &[ParameterizedValue<'a>]) -> my::Params {
    if params.len() > 0 {
        my::Params::Positional(params.iter().map(|x| x.into()).collect::<Vec<my::Value>>())
    } else {
        // If we don't use explicit 'Empty',
        // mysql crashes with 'internal error: entered unreachable code'
        my::Params::Empty
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
                my::Value::Date(year, month, day, hour, min, sec, _) => {
                    let naive = NaiveDate::from_ymd(*year as i32, *month as u32, *day as u32)
                        .and_hms(*hour as u32, *min as u32, *sec as u32);

                    let dt: DateTime<Utc> = DateTime::from_utc(naive, Utc);
                    ParameterizedValue::DateTime(dt)
                }
                #[cfg(feature = "chrono-0_4")]
                my::Value::Time(is_neg, days, hours, minutes, seconds, micros) => {
                    let days = Duration::days(*days as i64);
                    let hours = Duration::hours(*hours as i64);
                    let minutes = Duration::minutes(*minutes as i64);
                    let seconds = Duration::seconds(*seconds as i64);
                    let micros = Duration::microseconds(*micros as i64);

                    let time = days
                        .checked_add(&hours)
                        .and_then(|t| t.checked_add(&minutes))
                        .and_then(|t| t.checked_add(&seconds))
                        .and_then(|t| t.checked_add(&micros))
                        .unwrap();

                    let duration = time.to_std().unwrap();
                    let f_time = duration.as_secs() as f64 + duration.subsec_micros() as f64 * 1e-6;

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
    fn to_column_names<'b>(&'b self) -> Vec<String> {
        let mut names = Vec::new();

        if let Some(columns) = self.columns_ref() {
            for column in columns {
                names.push(String::from(column.name_str()));
            }
        }

        names
    }
}
