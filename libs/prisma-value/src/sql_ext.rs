use crate::PrismaValue;
use quaint::ast::Value;

impl<'a> From<Value<'a>> for PrismaValue {
    fn from(pv: Value<'a>) -> Self {
        match pv {
            Value::Null => PrismaValue::Null,
            Value::Integer(i) => PrismaValue::Int(i),
            Value::Real(d) => PrismaValue::Float(d),
            Value::Text(s) => PrismaValue::String(s.into_owned()),
            Value::Enum(s) => PrismaValue::Enum(s.into_owned()),
            Value::Boolean(b) => PrismaValue::Boolean(b),
            Value::Array(v) => PrismaValue::List(v.into_iter().map(PrismaValue::from).collect()),
            Value::Json(val) => PrismaValue::Json(val.to_string()),
            Value::Uuid(uuid) => PrismaValue::Uuid(uuid),
            Value::DateTime(dt) => PrismaValue::DateTime(dt),
            Value::Char(c) => PrismaValue::String(c.to_string()),
            Value::Bytes(bytes) => {
                let s = String::from_utf8(bytes.into_owned()).expect("PrismaValue::String from Value::Bytes");

                PrismaValue::String(s)
            }
        }
    }
}

impl<'a> From<PrismaValue> for Value<'a> {
    fn from(pv: PrismaValue) -> Self {
        match pv {
            PrismaValue::String(s) => s.into(),
            PrismaValue::Float(f) => f.into(),
            PrismaValue::Boolean(b) => b.into(),
            PrismaValue::DateTime(d) => d.into(),
            PrismaValue::Enum(e) => Value::Enum(e.into()),
            PrismaValue::Int(i) => (i as i64).into(),
            PrismaValue::Null => Value::Null,
            PrismaValue::Uuid(u) => u.to_string().into(),
            PrismaValue::List(l) => Value::Array(l.into_iter().map(|x| x.into()).collect()),
            PrismaValue::Json(s) => Value::Text(s.into()),
        }
    }
}
