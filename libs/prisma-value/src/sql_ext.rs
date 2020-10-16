use crate::PrismaValue;
use chrono::{DateTime, NaiveDate, Utc};
use quaint::ast::Value;

impl<'a> From<Value<'a>> for PrismaValue {
    fn from(pv: Value<'a>) -> Self {
        match pv {
            Value::Integer(i) => i.map(PrismaValue::Int).unwrap_or(PrismaValue::Null),
            Value::Real(d) => d
                // chop the trailing zeroes off so javascript doesn't start rounding things wrong
                .map(|d| PrismaValue::Float(d.normalize()))
                .unwrap_or(PrismaValue::Null),
            Value::Text(s) => s
                .map(|s| PrismaValue::String(s.into_owned()))
                .unwrap_or(PrismaValue::Null),
            Value::Enum(s) => s
                .map(|s| PrismaValue::Enum(s.into_owned()))
                .unwrap_or(PrismaValue::Null),
            Value::Boolean(b) => b.map(PrismaValue::Boolean).unwrap_or(PrismaValue::Null),
            Value::Array(v) => v
                .map(|v| PrismaValue::List(v.into_iter().map(PrismaValue::from).collect()))
                .unwrap_or(PrismaValue::Null),
            Value::Json(val) => val
                .map(|val| PrismaValue::Json(val.to_string()))
                .unwrap_or(PrismaValue::Null),
            Value::Uuid(uuid) => uuid.map(PrismaValue::Uuid).unwrap_or(PrismaValue::Null),
            Value::Date(d) => d
                .map(|d| {
                    let dt = DateTime::<Utc>::from_utc(d.and_hms(0, 0, 0), Utc);
                    PrismaValue::DateTime(dt)
                })
                .unwrap_or(PrismaValue::Null),
            Value::Time(t) => t
                .map(|t| {
                    let d = NaiveDate::from_ymd(1970, 1, 1);
                    let dt = DateTime::<Utc>::from_utc(d.and_time(t), Utc);
                    PrismaValue::DateTime(dt)
                })
                .unwrap_or(PrismaValue::Null),
            Value::DateTime(dt) => dt.map(PrismaValue::DateTime).unwrap_or(PrismaValue::Null),
            Value::Char(c) => c
                .map(|c| PrismaValue::String(c.to_string()))
                .unwrap_or(PrismaValue::Null),
            Value::Bytes(bytes) => bytes
                .map(|bytes| {
                    let s = String::from_utf8(bytes.into_owned()).expect("PrismaValue::String from Value::Bytes");
                    PrismaValue::String(s)
                })
                .unwrap_or(PrismaValue::Null),
            Value::Xml(_) => todo!(),
        }
    }
}
