use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use prisma_value::{PlaceholderType, PrismaValue};
use quaint::ast::VarType;

pub(crate) fn quaint_value_to_prisma_value(value: quaint::Value<'_>) -> PrismaValue {
    match value.typed {
        quaint::ValueType::Int32(Some(i)) => PrismaValue::Int(i.into()),
        quaint::ValueType::Int32(None) => PrismaValue::Null,
        quaint::ValueType::Int64(Some(i)) => PrismaValue::BigInt(i),
        quaint::ValueType::Int64(None) => PrismaValue::Null,
        quaint::ValueType::Float(Some(f)) => PrismaValue::Float(
            BigDecimal::from_f32(f)
                .expect("float to decimal conversion should succeed")
                .normalized(),
        ),
        quaint::ValueType::Float(None) => PrismaValue::Null,
        quaint::ValueType::Double(Some(d)) => PrismaValue::Float(
            BigDecimal::from_f64(d)
                .expect("double to decimal conversion should succeed")
                .normalized(),
        ),
        quaint::ValueType::Double(None) => PrismaValue::Null,
        quaint::ValueType::Text(Some(s)) => PrismaValue::String(s.into_owned()),
        quaint::ValueType::Text(None) => PrismaValue::Null,
        quaint::ValueType::Enum(Some(e), _) => PrismaValue::Enum(e.into_owned()),
        quaint::ValueType::Enum(None, _) => PrismaValue::Null,
        quaint::ValueType::EnumArray(Some(es), _) => PrismaValue::List(
            es.into_iter()
                .map(|e| e.into_text())
                .map(quaint_value_to_prisma_value)
                .collect(),
        ),
        quaint::ValueType::EnumArray(None, _) => PrismaValue::Null,
        quaint::ValueType::Bytes(Some(b)) => PrismaValue::Bytes(b.into_owned()),
        quaint::ValueType::Bytes(None) => PrismaValue::Null,
        quaint::ValueType::Boolean(Some(b)) => PrismaValue::Boolean(b),
        quaint::ValueType::Boolean(None) => PrismaValue::Null,
        quaint::ValueType::Char(Some(c)) => PrismaValue::String(c.to_string()),
        quaint::ValueType::Char(None) => PrismaValue::Null,
        quaint::ValueType::Array(Some(a)) => {
            PrismaValue::List(a.into_iter().map(quaint_value_to_prisma_value).collect())
        }
        quaint::ValueType::Array(None) => PrismaValue::Null,
        quaint::ValueType::Numeric(Some(bd)) => PrismaValue::Float(bd),
        quaint::ValueType::Numeric(None) => PrismaValue::Null,
        quaint::ValueType::Json(Some(j)) => PrismaValue::Json(j.to_string()),
        quaint::ValueType::Json(None) => PrismaValue::Null,
        quaint::ValueType::Xml(Some(x)) => PrismaValue::String(x.into_owned()),
        quaint::ValueType::Xml(None) => PrismaValue::Null,
        quaint::ValueType::Uuid(Some(u)) => PrismaValue::Uuid(u),
        quaint::ValueType::Uuid(None) => PrismaValue::Null,
        quaint::ValueType::DateTime(Some(dt)) => PrismaValue::DateTime(dt.into()),
        quaint::ValueType::DateTime(None) => PrismaValue::Null,
        quaint::ValueType::Date(Some(d)) => {
            let dt = DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc);
            PrismaValue::DateTime(dt.into())
        }
        quaint::ValueType::Date(None) => PrismaValue::Null,
        quaint::ValueType::Time(Some(t)) => {
            let d = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let dt = DateTime::<Utc>::from_naive_utc_and_offset(d.and_time(t), Utc);
            PrismaValue::DateTime(dt.into())
        }
        quaint::ValueType::Time(None) => PrismaValue::Null,
        quaint::ValueType::Var(name, vt) => PrismaValue::Placeholder {
            name: name.into_owned(),
            r#type: var_type_to_placeholder_type(&vt),
        },
    }
}

fn var_type_to_placeholder_type(vt: &VarType) -> PlaceholderType {
    match vt {
        VarType::Unknown => PlaceholderType::Any,
        VarType::Int32 => PlaceholderType::Int,
        VarType::Int64 => PlaceholderType::BigInt,
        VarType::Float => PlaceholderType::Float,
        VarType::Double => PlaceholderType::Float,
        VarType::Text => PlaceholderType::String,
        VarType::Enum => PlaceholderType::String,
        VarType::Bytes => PlaceholderType::Bytes,
        VarType::Boolean => PlaceholderType::Boolean,
        VarType::Char => PlaceholderType::String,
        VarType::Array(t) => PlaceholderType::Array(Box::new(var_type_to_placeholder_type(t))),
        VarType::Numeric => PlaceholderType::Float,
        VarType::Json => PlaceholderType::Object,
        VarType::Xml => PlaceholderType::String,
        VarType::Uuid => PlaceholderType::String,
        VarType::DateTime => PlaceholderType::Date,
        VarType::Date => PlaceholderType::Date,
        VarType::Time => PlaceholderType::Date,
    }
}
