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
            (PrismaValue::DateTime(d), _) => d.into(),
            (PrismaValue::Enum(e), _) => e.into(),
            (PrismaValue::Int(i), _) => (i as i64).into(),
            (PrismaValue::Uuid(u), _) => u.to_string().into(),
            (PrismaValue::List(l), _) => Value::Array(Some(l.into_iter().map(|x| self.value(x)).collect())),
            (PrismaValue::Json(s), _) => Value::Json(serde_json::from_str(&s).unwrap()),
            (PrismaValue::Null, ident) => match ident {
                _ if self.is_list => Value::Array(None),
                TypeIdentifier::String => Value::Text(None),
                TypeIdentifier::Float => Value::Real(None),
                TypeIdentifier::Boolean => Value::Boolean(None),
                TypeIdentifier::Enum(_) => Value::Enum(None),
                TypeIdentifier::Json => Value::Json(None),
                TypeIdentifier::DateTime => Value::DateTime(None),
                TypeIdentifier::UUID => Value::Uuid(None),
                TypeIdentifier::Int => Value::Integer(None),
            },
        }
    }
}
