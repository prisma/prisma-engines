use chrono::{DateTime, NaiveDate, Utc};
use quaint::ValueType;
use query_structure::PrismaValue;

pub fn to_prisma_value<'a, T: Into<ValueType<'a>>>(qv: T) -> crate::Result<PrismaValue> {
    let val = match qv.into() {
        ValueType::Int32(i) => i.map(|i| PrismaValue::Int(i as i64)).unwrap_or(PrismaValue::Null),
        ValueType::Int64(i) => i.map(PrismaValue::Int).unwrap_or(PrismaValue::Null),

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
                let dt = DateTime::<Utc>::from_utc(d.and_hms_opt(0, 0, 0).unwrap(), Utc);
                PrismaValue::DateTime(dt.into())
            })
            .unwrap_or(PrismaValue::Null),

        ValueType::Time(t) => t
            .map(|t| {
                let d = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                let dt = DateTime::<Utc>::from_utc(d.and_time(t), Utc);
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
    };

    Ok(val)
}
