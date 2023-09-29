use crate::row::{sanitize_f32, sanitize_f64};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use prisma_models::PrismaValue;
use quaint::Value;

pub fn to_prisma_value(quaint_value: Value<'_>) -> crate::Result<PrismaValue> {
    let val = match quaint_value {
        Value::Int32(i) => i.map(|i| PrismaValue::Int(i as i64)).unwrap_or(PrismaValue::Null),
        Value::Int64(i) => i.map(PrismaValue::Int).unwrap_or(PrismaValue::Null),
        Value::Float(Some(f)) => {
            sanitize_f32(f, "BigDecimal")?;

            PrismaValue::Float(BigDecimal::from_f32(f).unwrap().normalized())
        }

        Value::Float(None) => PrismaValue::Null,

        Value::Double(Some(f)) => {
            sanitize_f64(f, "BigDecimal")?;

            PrismaValue::Float(BigDecimal::from_f64(f).unwrap().normalized())
        }

        Value::Double(None) => PrismaValue::Null,

        Value::Numeric(d) => d
            // chop the trailing zeroes off so javascript doesn't start rounding things wrong
            .map(|d| PrismaValue::Float(d.normalized()))
            .unwrap_or(PrismaValue::Null),

        Value::Text(s) => s
            .map(|s| PrismaValue::String(s.into_owned()))
            .unwrap_or(PrismaValue::Null),

        Value::Enum(s, _) => s
            .map(|s| PrismaValue::Enum(s.into_owned()))
            .unwrap_or(PrismaValue::Null),

        Value::Boolean(b) => b.map(PrismaValue::Boolean).unwrap_or(PrismaValue::Null),

        Value::Array(Some(v)) => {
            let mut res = Vec::with_capacity(v.len());

            for v in v.into_iter() {
                res.push(to_prisma_value(v)?);
            }

            PrismaValue::List(res)
        }

        Value::Array(None) => PrismaValue::Null,

        Value::EnumArray(Some(v), name) => {
            let mut res = Vec::with_capacity(v.len());

            for v in v.into_iter() {
                res.push(to_prisma_value(Value::Enum(Some(v), name.clone()))?);
            }

            PrismaValue::List(res)
        }
        Value::EnumArray(None, _) => PrismaValue::Null,

        Value::Json(val) => val
            .map(|val| PrismaValue::Json(val.to_string()))
            .unwrap_or(PrismaValue::Null),

        Value::Uuid(uuid) => uuid.map(PrismaValue::Uuid).unwrap_or(PrismaValue::Null),

        Value::Date(d) => d
            .map(|d| {
                let dt = DateTime::<Utc>::from_utc(d.and_hms_opt(0, 0, 0).unwrap(), Utc);
                PrismaValue::DateTime(dt.into())
            })
            .unwrap_or(PrismaValue::Null),

        Value::Time(t) => t
            .map(|t| {
                let d = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
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
            .map(|b| PrismaValue::Bytes(b.into_owned()))
            .unwrap_or(PrismaValue::Null),

        Value::Xml(s) => s
            .map(|s| PrismaValue::String(s.into_owned()))
            .unwrap_or(PrismaValue::Null),
    };

    Ok(val)
}
