use crate::PrismaValue;
use quaint::ast::ParameterizedValue;

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
            ParameterizedValue::Bytes(_bytes) => unreachable!("PrismaValue::Bytes"),
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
            PrismaValue::List(l) => ParameterizedValue::Array(l.into_iter().map(|x| x.into()).collect()),
        }
    }
}
