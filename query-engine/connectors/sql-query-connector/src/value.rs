use crate::row::{sanitize_f32, sanitize_f64};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use prisma_models::PrismaValue;
use quaint::ValueInner;

pub fn to_prisma_value(quaint_value_type: ValueInner<'_>) -> crate::Result<PrismaValue> {
    let val = match quaint_value_type {
        ValueInner::Int32(i) => i.map(|i| PrismaValue::Int(i as i64)).unwrap_or(PrismaValue::Null),
        ValueInner::Int64(i) => i.map(PrismaValue::Int).unwrap_or(PrismaValue::Null),
        ValueInner::Float(Some(f)) => {
            sanitize_f32(f, "BigDecimal")?;

            PrismaValue::Float(BigDecimal::from_f32(f).unwrap().normalized())
        }

        ValueInner::Float(None) => PrismaValue::Null,

        ValueInner::Double(Some(f)) => {
            sanitize_f64(f, "BigDecimal")?;

            PrismaValue::Float(BigDecimal::from_f64(f).unwrap().normalized())
        }

        ValueInner::Double(None) => PrismaValue::Null,

        ValueInner::Numeric(d) => d
            // chop the trailing zeroes off so javascript doesn't start rounding things wrong
            .map(|d| PrismaValue::Float(d.normalized()))
            .unwrap_or(PrismaValue::Null),

        ValueInner::Text(s) => s
            .map(|s| PrismaValue::String(s.into_owned()))
            .unwrap_or(PrismaValue::Null),

        ValueInner::Enum(s, _) => s
            .map(|s| PrismaValue::Enum(s.into_owned()))
            .unwrap_or(PrismaValue::Null),

        ValueInner::Boolean(b) => b.map(PrismaValue::Boolean).unwrap_or(PrismaValue::Null),

        ValueInner::Array(Some(v)) => {
            let mut res = Vec::with_capacity(v.len());

            for v in v.into_iter() {
                res.push(to_prisma_value(v.into())?);
            }

            PrismaValue::List(res)
        }

        ValueInner::Array(None) => PrismaValue::Null,

        ValueInner::EnumArray(Some(v), name) => {
            let mut res = Vec::with_capacity(v.len());

            for v in v.into_iter() {
                res.push(to_prisma_value(ValueInner::Enum(Some(v), name.clone()).into())?);
            }

            PrismaValue::List(res)
        }
        ValueInner::EnumArray(None, _) => PrismaValue::Null,

        ValueInner::Json(val) => val
            .map(|val| PrismaValue::Json(val.to_string()))
            .unwrap_or(PrismaValue::Null),

        ValueInner::Uuid(uuid) => uuid.map(PrismaValue::Uuid).unwrap_or(PrismaValue::Null),

        ValueInner::Date(d) => d
            .map(|d| {
                let dt = DateTime::<Utc>::from_utc(d.and_hms_opt(0, 0, 0).unwrap(), Utc);
                PrismaValue::DateTime(dt.into())
            })
            .unwrap_or(PrismaValue::Null),

        ValueInner::Time(t) => t
            .map(|t| {
                let d = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                let dt = DateTime::<Utc>::from_utc(d.and_time(t), Utc);
                PrismaValue::DateTime(dt.into())
            })
            .unwrap_or(PrismaValue::Null),

        ValueInner::DateTime(dt) => dt
            .map(|dt| PrismaValue::DateTime(dt.into()))
            .unwrap_or(PrismaValue::Null),

        ValueInner::Char(c) => c
            .map(|c| PrismaValue::String(c.to_string()))
            .unwrap_or(PrismaValue::Null),

        ValueInner::Bytes(bytes) => bytes
            .map(|b| PrismaValue::Bytes(b.into_owned()))
            .unwrap_or(PrismaValue::Null),

        ValueInner::Xml(s) => s
            .map(|s| PrismaValue::String(s.into_owned()))
            .unwrap_or(PrismaValue::Null),
    };

    Ok(val)
}
