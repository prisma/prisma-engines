use crate::{ast::Value, connector::queryable::TakeRow, error::ErrorKind};
#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use mysql_async as my;
use mysql_async::Value as MyValue;
use rust_decimal::prelude::ToPrimitive;
use std::convert::TryFrom;

pub fn conv_params<'a>(params: &[Value<'a>]) -> my::Params {
    if params.is_empty() {
        // If we don't use explicit 'Empty',
        // mysql crashes with 'internal error: entered unreachable code'
        my::Params::Empty
    } else {
        my::Params::Positional(params.iter().map(|x| x.into()).collect::<Vec<my::Value>>())
    }
}

impl TakeRow for my::Row {
    fn take_result_row<'b>(&'b mut self) -> crate::Result<Vec<Value<'static>>> {
        fn convert(row: &mut my::Row, i: usize) -> crate::Result<Value<'static>> {
            use mysql_async::consts::ColumnType::*;

            let value = row.take(i).ok_or_else(|| {
                crate::error::Error::builder(ErrorKind::ConversionError("Index out of bounds")).build()
            })?;

            let column = row.columns_ref().get(i).ok_or_else(|| {
                crate::error::Error::builder(ErrorKind::ConversionError("Index out of bounds")).build()
            })?;

            let res = match value {
                // JSON is returned as bytes.
                #[cfg(feature = "json-1")]
                my::Value::Bytes(b) if column.column_type() == MYSQL_TYPE_JSON => {
                    serde_json::from_slice(&b).map(|val| Value::json(val)).map_err(|_e| {
                        crate::error::Error::builder(ErrorKind::ConversionError("Unable to convert bytes to JSON"))
                            .build()
                    })?
                }
                // NEWDECIMAL returned as bytes. See https://mariadb.com/kb/en/resultset-row/#decimal-binary-encoding
                my::Value::Bytes(b) if column.column_type() == MYSQL_TYPE_NEWDECIMAL => Value::real(
                    String::from_utf8(b)
                        .expect("MySQL NEWDECIMAL as string")
                        .parse()
                        .map_err(|_err| {
                            crate::error::Error::builder(ErrorKind::ConversionError("mysql NEWDECIMAL conversion"))
                                .build()
                        })?,
                ),
                // https://dev.mysql.com/doc/internals/en/character-set.html
                my::Value::Bytes(b) if column.character_set() == 63 => Value::bytes(b),
                my::Value::Bytes(s) => Value::text(String::from_utf8(s)?),
                my::Value::Int(i) => Value::integer(i),
                my::Value::UInt(i) => Value::integer(i64::try_from(i).map_err(|_| {
                    let builder = crate::error::Error::builder(ErrorKind::ValueOutOfRange {
                        message: "Unsigned integers larger than 9_223_372_036_854_775_807 are currently not handled."
                            .into(),
                    });
                    builder.build()
                })?),
                my::Value::Float(f) => Value::from(f),
                my::Value::Double(f) => Value::from(f),
                #[cfg(feature = "chrono-0_4")]
                my::Value::Date(year, month, day, hour, min, sec, micro) => {
                    let time = NaiveTime::from_hms_micro(hour.into(), min.into(), sec.into(), micro);

                    let date = NaiveDate::from_ymd(year.into(), month.into(), day.into());
                    let dt = NaiveDateTime::new(date, time);

                    Value::datetime(DateTime::<Utc>::from_utc(dt, Utc))
                }
                #[cfg(feature = "chrono-0_4")]
                my::Value::Time(is_neg, days, hours, minutes, seconds, micros) => {
                    if is_neg {
                        let kind = ErrorKind::ConversionError("Failed to convert a negative time");
                        return Err(crate::error::Error::builder(kind).build());
                    }

                    if days != 0 {
                        let kind = ErrorKind::ConversionError("Failed to read a MySQL `time` as duration");
                        return Err(crate::error::Error::builder(kind).build());
                    }

                    let time = NaiveTime::from_hms_micro(hours.into(), minutes.into(), seconds.into(), micros);
                    Value::time(time)
                }
                my::Value::NULL => match column.column_type() {
                    MYSQL_TYPE_DECIMAL | MYSQL_TYPE_FLOAT | MYSQL_TYPE_DOUBLE | MYSQL_TYPE_NEWDECIMAL => {
                        Value::Real(None)
                    }
                    MYSQL_TYPE_NULL => Value::Integer(None),
                    MYSQL_TYPE_TINY | MYSQL_TYPE_SHORT | MYSQL_TYPE_LONG | MYSQL_TYPE_LONGLONG => Value::Integer(None),
                    #[cfg(feature = "chrono-0_4")]
                    MYSQL_TYPE_TIMESTAMP
                    | MYSQL_TYPE_TIME
                    | MYSQL_TYPE_DATE
                    | MYSQL_TYPE_DATETIME
                    | MYSQL_TYPE_YEAR
                    | MYSQL_TYPE_NEWDATE
                    | MYSQL_TYPE_TIMESTAMP2
                    | MYSQL_TYPE_DATETIME2
                    | MYSQL_TYPE_TIME2 => Value::DateTime(None),
                    MYSQL_TYPE_VARCHAR | MYSQL_TYPE_VAR_STRING | MYSQL_TYPE_STRING => Value::Text(None),
                    MYSQL_TYPE_BIT => Value::Boolean(None),
                    #[cfg(feature = "json-1")]
                    MYSQL_TYPE_JSON => Value::Json(None),
                    MYSQL_TYPE_ENUM => Value::Enum(None),
                    MYSQL_TYPE_TINY_BLOB | MYSQL_TYPE_MEDIUM_BLOB | MYSQL_TYPE_LONG_BLOB | MYSQL_TYPE_BLOB
                        if column.character_set() == 63 =>
                    {
                        Value::Bytes(None)
                    }
                    MYSQL_TYPE_TINY_BLOB | MYSQL_TYPE_MEDIUM_BLOB | MYSQL_TYPE_LONG_BLOB | MYSQL_TYPE_BLOB => {
                        Value::Text(None)
                    }
                    typ => panic!(
                        "Value of type {:?} is not supported with the current configuration",
                        typ
                    ),
                },
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

impl<'a> From<Value<'a>> for MyValue {
    fn from(pv: Value<'a>) -> MyValue {
        let res = match pv {
            Value::Integer(i) => i.map(|i| MyValue::Int(i)),
            Value::Real(f) => f.map(|f| MyValue::Double(f.to_f64().expect("Decimal is not a f64."))),
            Value::Text(s) => s.map(|s| MyValue::Bytes((&*s).as_bytes().to_vec())),
            Value::Bytes(bytes) => bytes.map(|bytes| MyValue::Bytes(bytes.into_owned())),
            Value::Enum(s) => s.map(|s| MyValue::Bytes((&*s).as_bytes().to_vec())),
            Value::Boolean(b) => b.map(|b| MyValue::Int(b as i64)),
            Value::Char(c) => c.map(|c| MyValue::Bytes(vec![c as u8])),
            #[cfg(feature = "json-1")]
            Value::Json(json) => json.map(|json| {
                let s = serde_json::to_string(&json).expect("Cannot convert JSON to String.");
                MyValue::Bytes(s.into_bytes())
            }),
            #[cfg(feature = "array")]
            Value::Array(_) => unimplemented!("Arrays are not supported for mysql."),
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(u) => u.map(|u| MyValue::Bytes(u.to_hyphenated().to_string().into_bytes())),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(d) => d.map(|d| MyValue::Date(d.year() as u16, d.month() as u8, d.day() as u8, 0, 0, 0, 0)),
            #[cfg(feature = "chrono-0_4")]
            Value::Time(t) => t.map(|t| MyValue::Time(false, 0, t.hour() as u8, t.minute() as u8, t.second() as u8, 0)),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(dt) => dt.map(|dt| {
                MyValue::Date(
                    dt.year() as u16,
                    dt.month() as u8,
                    dt.day() as u8,
                    dt.hour() as u8,
                    dt.minute() as u8,
                    dt.second() as u8,
                    dt.timestamp_subsec_micros(),
                )
            }),
        };

        match res {
            Some(val) => val,
            None => MyValue::NULL,
        }
    }
}
