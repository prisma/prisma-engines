use crate::{ConversionFailure, GraphqlId, PrismaValue};
use quaint::ast::{DatabaseValue, ParameterizedValue};
use std::convert::TryFrom;

impl<'a> TryFrom<ParameterizedValue<'a>> for GraphqlId {
    type Error = ConversionFailure;

    fn try_from(pv: ParameterizedValue<'a>) -> Result<Self, Self::Error> {
        match pv {
            ParameterizedValue::Integer(i) => Ok(GraphqlId::Int(i as usize)),
            ParameterizedValue::Text(s) => Ok(GraphqlId::String(s.into_owned())),
            ParameterizedValue::Uuid(uuid) => Ok(GraphqlId::UUID(uuid)),
            _ => Err(ConversionFailure::new("ParameterizedValue", "GraphqlId")),
        }
    }
}

impl<'a> From<GraphqlId> for ParameterizedValue<'a> {
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

impl<'a> From<ParameterizedValue<'a>> for PrismaValue {
    fn from(pv: ParameterizedValue<'a>) -> Self {
        match pv {
            ParameterizedValue::Null => PrismaValue::Null,
            ParameterizedValue::Integer(i) => PrismaValue::Int(i),
            ParameterizedValue::Real(d) => PrismaValue::Float(d),
            ParameterizedValue::Text(s) => PrismaValue::String(s.into_owned()),
            ParameterizedValue::Enum(s) => PrismaValue::Enum(s.into_owned()),
            ParameterizedValue::Boolean(b) => PrismaValue::Boolean(b),
            ParameterizedValue::Array(v) => PrismaValue::List(v.into_iter().map(PrismaValue::from).collect()),
            ParameterizedValue::Json(val) => PrismaValue::String(val.to_string()),
            ParameterizedValue::Uuid(uuid) => PrismaValue::Uuid(uuid),
            ParameterizedValue::DateTime(dt) => PrismaValue::DateTime(dt),
            ParameterizedValue::Char(c) => PrismaValue::String(c.to_string()),
            ParameterizedValue::Enum(e) => PrismaValue::Enum(e.into_owned()),
        }
    }
}

impl<'a> From<PrismaValue> for ParameterizedValue<'a> {
    fn from(pv: PrismaValue) -> Self {
        match pv {
            PrismaValue::String(s) => s.into(),
            PrismaValue::Float(f) => f.into(),
            PrismaValue::Boolean(b) => b.into(),
            PrismaValue::DateTime(d) => d.into(),
            PrismaValue::Enum(e) => ParameterizedValue::Enum(e.into()),
            PrismaValue::Int(i) => (i as i64).into(),
            PrismaValue::Null => ParameterizedValue::Null,
            PrismaValue::Uuid(u) => u.to_string().into(),
            PrismaValue::GraphqlId(id) => id.into(),
            PrismaValue::List(l) => ParameterizedValue::Array(l.into_iter().map(|x| x.into()).collect()),
        }
    }
}
