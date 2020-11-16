use chrono::Utc;
use prisma_value::PrismaValue;
use quaint::ast::Value;

use crate::{ScalarField, TypeIdentifier};

pub trait ScalarFieldExt {
    fn value<'a>(&self, pv: PrismaValue) -> Value<'a>;
}

impl ScalarFieldExt for ScalarField {
    fn value<'a>(&self, pv: PrismaValue) -> Value<'a> {
        match (pv, &self.type_identifier) {
            (PrismaValue::String(s), _) => s.into(),
            (PrismaValue::Float(f), _) => f.into(),
            (PrismaValue::Boolean(b), _) => b.into(),
            (PrismaValue::DateTime(d), _) => d.with_timezone(&Utc).into(),
            (PrismaValue::Enum(e), _) => e.into(),
            (PrismaValue::Int(i), _) => (i as i64).into(),
            (PrismaValue::BigInt(i), _) => (i as i64).into(),
            (PrismaValue::Uuid(u), _) => u.to_string().into(),
            (PrismaValue::List(l), _) => Value::Array(Some(l.into_iter().map(|x| self.value(x)).collect())),
            (PrismaValue::Json(s), _) => Value::Json(serde_json::from_str(&s).unwrap()),
            (PrismaValue::Bytes(b), _) => Value::Bytes(Some(b.into())),
            (PrismaValue::Xml(s), _) => Value::Xml(Some(s.into())),
            (PrismaValue::Null, ident) => match ident {
                TypeIdentifier::String => Value::Text(None),
                TypeIdentifier::Float => Value::Numeric(None),
                TypeIdentifier::Decimal => Value::Numeric(None),
                TypeIdentifier::Boolean => Value::Boolean(None),
                TypeIdentifier::Enum(_) => Value::Enum(None),
                TypeIdentifier::Json => Value::Json(None),
                TypeIdentifier::DateTime => Value::DateTime(None),
                TypeIdentifier::UUID => Value::Uuid(None),
                TypeIdentifier::Int => Value::Integer(None),
                TypeIdentifier::BigInt => Value::Integer(None),
                TypeIdentifier::Bytes => Value::Bytes(None),
                TypeIdentifier::Xml => Value::Xml(None),
            },
        }
    }
}

/// Attempts to convert a PrismaValue to a database value without any additional type information.
/// Can't reliably map Null values.
pub fn convert_lossy<'a>(pv: PrismaValue) -> Value<'a> {
    match pv {
        PrismaValue::String(s) => s.into(),
        PrismaValue::Float(f) => f.into(),
        PrismaValue::Boolean(b) => b.into(),
        PrismaValue::DateTime(d) => d.with_timezone(&Utc).into(),
        PrismaValue::Enum(e) => e.into(),
        PrismaValue::Int(i) => (i as i64).into(),
        PrismaValue::BigInt(i) => (i as i64).into(),
        PrismaValue::Uuid(u) => u.to_string().into(),
        PrismaValue::List(l) => Value::Array(Some(l.into_iter().map(convert_lossy).collect())),
        PrismaValue::Json(s) => Value::Json(serde_json::from_str(&s).unwrap()),
        PrismaValue::Bytes(b) => Value::Bytes(Some(b.into())),
        PrismaValue::Xml(s) => Value::Xml(Some(s.into())),
        PrismaValue::Null => Value::Integer(None), // Can't tell which type the null is supposed to be.
    }
}
