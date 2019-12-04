use crate::{GraphqlId, PrismaValue};
use quaint::ast::{DatabaseValue, Id, ParameterizedValue};

impl From<Id> for GraphqlId {
    fn from(id: Id) -> Self {
        match id {
            Id::String(s) => GraphqlId::String(s),
            Id::Int(i) => GraphqlId::Int(i),
            Id::UUID(u) => GraphqlId::UUID(u),
        }
    }
}

impl<'a> From<GraphqlId> for DatabaseValue<'a> {
    fn from(id: GraphqlId) -> Self {
        match id {
            GraphqlId::String(s) => s.into(),
            GraphqlId::Int(i) => (i as i64).into(),
            GraphqlId::UUID(u) => u.to_string().into(),
        }
    }
}

impl<'a> From<&GraphqlId> for DatabaseValue<'a> {
    fn from(id: &GraphqlId) -> Self {
        id.clone().into()
    }
}

impl<'a> From<PrismaValue> for DatabaseValue<'a> {
    fn from(pv: PrismaValue) -> Self {
        match pv {
            PrismaValue::String(s) => s.into(),
            PrismaValue::Float(f) => (f as f64).into(),
            PrismaValue::Boolean(b) => b.into(),
            PrismaValue::DateTime(d) => d.into(),
            PrismaValue::Enum(e) => e.as_string().into(),
            PrismaValue::Json(j) => j.to_string().into(),
            PrismaValue::Int(i) => (i as i64).into(),
            PrismaValue::Null => DatabaseValue::Parameterized(ParameterizedValue::Null),
            PrismaValue::Uuid(u) => u.to_string().into(),
            PrismaValue::GraphqlId(id) => id.into(),
            PrismaValue::List(Some(l)) => l.into(),
            PrismaValue::List(_) => panic!("List values are not supported here"),
        }
    }
}

impl<'a> From<ParameterizedValue<'a>> for PrismaValue {
    fn from(pv: ParameterizedValue<'a>) -> Self {
        match pv {
            ParameterizedValue::Null => PrismaValue::Null,
            ParameterizedValue::Integer(i) => PrismaValue::Int(i),
            ParameterizedValue::Real(f) => PrismaValue::Float(f),
            ParameterizedValue::Text(s) => PrismaValue::String(s.into_owned()),
            ParameterizedValue::Boolean(b) => PrismaValue::Boolean(b),
            ParameterizedValue::Array(v) => {
                let lst = v.into_iter().map(PrismaValue::from).collect();
                PrismaValue::List(Some(lst))
            }
            ParameterizedValue::Json(val) => PrismaValue::Json(val),
            ParameterizedValue::Uuid(uuid) => PrismaValue::Uuid(uuid),
            ParameterizedValue::DateTime(dt) => PrismaValue::DateTime(dt),
            ParameterizedValue::Char(c) => PrismaValue::String(c.to_string()),
        }
    }
}
