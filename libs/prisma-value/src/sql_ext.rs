use crate::{PrismaValue, TypeHint};
use chrono::{DateTime, NaiveDate, Utc};
use quaint::ast::Value;

impl<'a> From<Value<'a>> for PrismaValue {
    fn from(pv: Value<'a>) -> Self {
        match pv {
            Value::Integer(i) => i
                .map(|i| PrismaValue::Int(i))
                .unwrap_or(PrismaValue::null(TypeHint::Int)),
            Value::Real(d) => d
                // chop the trailing zeroes off so javascript doesn't start rounding things wrong
                .map(|d| PrismaValue::Float(d.normalize()))
                .unwrap_or(PrismaValue::null(TypeHint::Float)),
            Value::Text(s) => s
                .map(|s| PrismaValue::String(s.into_owned()))
                .unwrap_or(PrismaValue::null(TypeHint::String)),
            Value::Enum(s) => s
                .map(|s| PrismaValue::Enum(s.into_owned()))
                .unwrap_or(PrismaValue::null(TypeHint::Enum)),
            Value::Boolean(b) => b
                .map(|b| PrismaValue::Boolean(b))
                .unwrap_or(PrismaValue::null(TypeHint::Boolean)),
            Value::Array(v) => v
                .map(|v| PrismaValue::List(v.into_iter().map(PrismaValue::from).collect()))
                .unwrap_or(PrismaValue::null(TypeHint::Array)),
            Value::Json(val) => val
                .map(|val| PrismaValue::Json(val.to_string()))
                .unwrap_or(PrismaValue::null(TypeHint::Json)),
            Value::Uuid(uuid) => uuid
                .map(|uuid| PrismaValue::Uuid(uuid))
                .unwrap_or(PrismaValue::null(TypeHint::UUID)),
            Value::Date(d) => d
                .map(|d| {
                    let dt = DateTime::<Utc>::from_utc(d.and_hms(0, 0, 0), Utc);
                    PrismaValue::DateTime(dt)
                })
                .unwrap_or(PrismaValue::null(TypeHint::DateTime)),
            Value::Time(t) => t
                .map(|t| {
                    let d = NaiveDate::from_ymd(1970, 1, 1);
                    let dt = DateTime::<Utc>::from_utc(d.and_time(t), Utc);
                    PrismaValue::DateTime(dt)
                })
                .unwrap_or(PrismaValue::null(TypeHint::DateTime)),
            Value::DateTime(dt) => dt
                .map(|dt| PrismaValue::DateTime(dt))
                .unwrap_or(PrismaValue::null(TypeHint::DateTime)),
            Value::Char(c) => c
                .map(|c| PrismaValue::String(c.to_string()))
                .unwrap_or(PrismaValue::null(TypeHint::Char)),
            Value::Bytes(bytes) => bytes
                .map(|bytes| {
                    let s = String::from_utf8(bytes.into_owned()).expect("PrismaValue::String from Value::Bytes");
                    PrismaValue::String(s)
                })
                .unwrap_or(PrismaValue::null(TypeHint::Bytes)),
        }
    }
}

impl<'a> From<PrismaValue> for Value<'a> {
    fn from(pv: PrismaValue) -> Self {
        match pv {
            PrismaValue::String(s) => s.into(),
            PrismaValue::Float(f) => f.into(),
            PrismaValue::Boolean(b) => b.into(),
            PrismaValue::DateTime(d) => d.into(),
            PrismaValue::Enum(e) => e.into(),
            PrismaValue::Int(i) => (i as i64).into(),
            PrismaValue::Uuid(u) => u.to_string().into(),
            PrismaValue::List(l) => Value::Array(Some(l.into_iter().map(|x| x.into()).collect())),
            PrismaValue::Json(s) => Value::Json(serde_json::from_str(&s).unwrap()),
            PrismaValue::Null(ident) => match ident {
                TypeHint::String => Value::Text(None),
                TypeHint::Float => Value::Real(None),
                TypeHint::Boolean => Value::Boolean(None),
                TypeHint::Enum => Value::Enum(None),
                TypeHint::Json => Value::Json(None),
                TypeHint::DateTime => Value::DateTime(None),
                TypeHint::UUID => Value::Uuid(None),
                TypeHint::Int => Value::Integer(None),
                TypeHint::Array => Value::Array(None),
                TypeHint::Char | TypeHint::Unknown => Value::Char(None),
                TypeHint::Bytes => Value::Bytes(None),
            },
        }
    }
}
