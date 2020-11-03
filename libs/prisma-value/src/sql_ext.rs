use crate::PrismaValue;
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use quaint::ast::Value;
use std::convert::TryFrom;

impl<'a> TryFrom<Value<'a>> for PrismaValue {
    type Error = crate::ConversionFailure;

    fn try_from(pv: Value<'a>) -> crate::PrismaValueResult<Self> {
        let val = match pv {
            Value::Integer(i) => i.map(PrismaValue::Int).unwrap_or(PrismaValue::Null),

            Value::Float(Some(f)) => match f {
                f if f.is_nan() => Err(crate::ConversionFailure {
                    from: "NaN",
                    to: "BigDecimal",
                })?,
                f if f.is_infinite() => Err(crate::ConversionFailure {
                    from: "Infinity",
                    to: "BigDecimal",
                })?,
                _ => PrismaValue::Float(BigDecimal::from_f32(f).unwrap().normalized()),
            },

            Value::Float(None) => PrismaValue::Null,

            Value::Double(Some(f)) => match f {
                f if f.is_nan() => Err(crate::ConversionFailure {
                    from: "NaN",
                    to: "BigDecimal",
                })?,
                f if f.is_infinite() => Err(crate::ConversionFailure {
                    from: "Infinity",
                    to: "BigDecimal",
                })?,
                _ => PrismaValue::Float(BigDecimal::from_f64(f).unwrap().normalized()),
            },

            Value::Double(None) => PrismaValue::Null,

            Value::Numeric(d) => d
                // chop the trailing zeroes off so javascript doesn't start rounding things wrong
                .map(|d| PrismaValue::Float(d.normalized()))
                .unwrap_or(PrismaValue::Null),

            Value::Text(s) => s
                .map(|s| PrismaValue::String(s.into_owned()))
                .unwrap_or(PrismaValue::Null),

            Value::Enum(s) => s
                .map(|s| PrismaValue::Enum(s.into_owned()))
                .unwrap_or(PrismaValue::Null),

            Value::Boolean(b) => b.map(PrismaValue::Boolean).unwrap_or(PrismaValue::Null),

            Value::Array(Some(v)) => {
                let mut res = Vec::with_capacity(v.len());

                for v in v.into_iter() {
                    res.push(PrismaValue::try_from(v)?);
                }

                PrismaValue::List(res)
            }

            Value::Array(None) => PrismaValue::Null,

            Value::Json(val) => val
                .map(|val| PrismaValue::Json(val.to_string()))
                .unwrap_or(PrismaValue::Null),

            Value::Uuid(uuid) => uuid.map(PrismaValue::Uuid).unwrap_or(PrismaValue::Null),

            Value::Date(d) => d
                .map(|d| {
                    let dt = DateTime::<Utc>::from_utc(d.and_hms(0, 0, 0), Utc);
                    PrismaValue::DateTime(dt.into())
                })
                .unwrap_or(PrismaValue::Null),

            Value::Time(t) => t
                .map(|t| {
                    let d = NaiveDate::from_ymd(1970, 1, 1);
                    let dt = DateTime::<Utc>::from_utc(d.and_time(t), Utc);
                    PrismaValue::DateTime(dt.into())
                })
                .unwrap_or(PrismaValue::Null),

            Value::DateTime(dt) => dt
                .map(|dt| PrismaValue::DateTime(dt.into()))
                .unwrap_or(PrismaValue::Null),

            Value::Char(c) => c
                .map(|c| PrismaValue::String(c.to_string()))
                .unwrap_or(PrismaValue::Null),

            Value::Bytes(bytes) => bytes
                .map(|bytes| {
                    let s = String::from_utf8(bytes.into_owned()).expect("PrismaValue::String from Value::Bytes");
                    PrismaValue::String(s)
                })
                .unwrap_or(PrismaValue::Null),

            Value::Xml(s) => s.map(|s| PrismaValue::Xml(s.into_owned())).unwrap_or(PrismaValue::Null),
        };

        Ok(val)
    }
}
