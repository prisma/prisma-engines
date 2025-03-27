use crate::row::{sanitize_f32, sanitize_f64};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use quaint::ValueType;
use query_structure::PrismaValue;
use sql_query_builder::{
    opaque_type_to_prisma_type,
    value::{GeneratorCall, Placeholder},
};

pub fn to_prisma_value<'a, T: Into<ValueType<'a>>>(qv: T) -> crate::Result<PrismaValue> {
    let val = match qv.into() {
        ValueType::Int32(i) => i.map(|i| PrismaValue::Int(i as i64)).unwrap_or(PrismaValue::Null),
        ValueType::Int64(i) => i.map(PrismaValue::Int).unwrap_or(PrismaValue::Null),
        ValueType::Float(Some(f)) => {
            sanitize_f32(f, "BigDecimal")?;

            PrismaValue::Float(BigDecimal::from_f32(f).unwrap().normalized())
        }

        ValueType::Float(None) => PrismaValue::Null,

        ValueType::Double(Some(f)) => {
            sanitize_f64(f, "BigDecimal")?;

            PrismaValue::Float(BigDecimal::from_f64(f).unwrap().normalized())
        }

        ValueType::Double(None) => PrismaValue::Null,

        ValueType::Numeric(d) => d
            // chop the trailing zeroes off so javascript doesn't start rounding things wrong
            .map(|d| PrismaValue::Float(d.normalized()))
            .unwrap_or(PrismaValue::Null),

        ValueType::Text(s) => s
            .map(|s| PrismaValue::String(s.into_owned()))
            .unwrap_or(PrismaValue::Null),

        ValueType::Enum(s, _) => s
            .map(|s| PrismaValue::Enum(s.into_owned()))
            .unwrap_or(PrismaValue::Null),

        ValueType::Boolean(b) => b.map(PrismaValue::Boolean).unwrap_or(PrismaValue::Null),

        ValueType::Array(Some(v)) => {
            let mut res = Vec::with_capacity(v.len());

            for v in v.into_iter() {
                res.push(to_prisma_value(v)?);
            }

            PrismaValue::List(res)
        }

        ValueType::Array(None) => PrismaValue::Null,

        ValueType::EnumArray(Some(v), name) => {
            let mut res = Vec::with_capacity(v.len());

            for v in v.into_iter() {
                res.push(to_prisma_value(ValueType::Enum(Some(v), name.clone()))?);
            }

            PrismaValue::List(res)
        }
        ValueType::EnumArray(None, _) => PrismaValue::Null,

        ValueType::Json(val) => val
            .map(|val| PrismaValue::Json(val.to_string()))
            .unwrap_or(PrismaValue::Null),

        ValueType::Uuid(uuid) => uuid.map(PrismaValue::Uuid).unwrap_or(PrismaValue::Null),

        ValueType::Date(d) => d
            .map(|d| {
                let dt = DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc);
                PrismaValue::DateTime(dt.into())
            })
            .unwrap_or(PrismaValue::Null),

        ValueType::Time(t) => t
            .map(|t| {
                let d = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                let dt = DateTime::<Utc>::from_naive_utc_and_offset(d.and_time(t), Utc);
                PrismaValue::DateTime(dt.into())
            })
            .unwrap_or(PrismaValue::Null),

        ValueType::DateTime(dt) => dt
            .map(|dt| PrismaValue::DateTime(dt.into()))
            .unwrap_or(PrismaValue::Null),

        ValueType::Char(c) => c
            .map(|c| PrismaValue::String(c.to_string()))
            .unwrap_or(PrismaValue::Null),

        ValueType::Bytes(bytes) => bytes
            .map(|b| PrismaValue::Bytes(b.into_owned()))
            .unwrap_or(PrismaValue::Null),

        ValueType::Xml(s) => s
            .map(|s| PrismaValue::String(s.into_owned()))
            .unwrap_or(PrismaValue::Null),

        ValueType::Opaque(opaque) => {
            if let Some(placeholder) = opaque.downcast_ref::<Placeholder>() {
                PrismaValue::Placeholder {
                    name: placeholder.name().to_owned(),
                    r#type: opaque_type_to_prisma_type(opaque.typ()),
                }
            } else if let Some(call) = opaque.downcast_ref::<GeneratorCall>() {
                PrismaValue::GeneratorCall {
                    name: call.name().to_owned(),
                    args: call.args().to_vec(),
                    return_type: opaque_type_to_prisma_type(opaque.typ()),
                }
            } else {
                unreachable!("invalid opaque value")
            }
        }
    };

    Ok(val)
}
