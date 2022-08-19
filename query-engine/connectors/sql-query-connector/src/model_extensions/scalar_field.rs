use chrono::Utc;
use prisma_models::{ScalarField, TypeIdentifier};
use prisma_value::PrismaValue;
use quaint::{
    ast::Value,
    prelude::{TypeDataLength, TypeFamily},
};

pub trait ScalarFieldExt {
    fn value<'a>(&self, pv: PrismaValue) -> Value<'a>;
    fn type_family(&self) -> TypeFamily;
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
            (PrismaValue::Json(s), _) => Value::Json(Some(serde_json::from_str::<serde_json::Value>(&s).unwrap())),
            (PrismaValue::Bytes(b), _) => Value::Bytes(Some(b.into())),
            (PrismaValue::Xml(s), _) => Value::Xml(Some(s.into())),
            (PrismaValue::Object(_), _) => unimplemented!(),
            (PrismaValue::Null, ident) => match ident {
                TypeIdentifier::String => Value::Text(None),
                TypeIdentifier::Float => Value::Numeric(None),
                TypeIdentifier::Decimal => Value::Numeric(None),
                TypeIdentifier::Boolean => Value::Boolean(None),
                TypeIdentifier::Enum(_) => Value::Enum(None),
                TypeIdentifier::Json => Value::Json(None),
                TypeIdentifier::DateTime => Value::DateTime(None),
                TypeIdentifier::UUID => Value::Uuid(None),
                TypeIdentifier::Int => Value::Int32(None),
                TypeIdentifier::BigInt => Value::Int64(None),
                TypeIdentifier::Bytes => Value::Bytes(None),
                TypeIdentifier::Xml => Value::Xml(None),
                TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
            },
        }
    }

    fn type_family(&self) -> TypeFamily {
        match self.type_identifier {
            TypeIdentifier::String => TypeFamily::Text(parse_scalar_length(self)),
            TypeIdentifier::Int => TypeFamily::Int,
            TypeIdentifier::BigInt => TypeFamily::Int,
            TypeIdentifier::Float => TypeFamily::Double,
            TypeIdentifier::Decimal => {
                let params = self
                    .native_type
                    .as_ref()
                    .map(|nt| nt.args.iter())
                    .and_then(|mut args| Some((args.next()?, args.next()?)))
                    .and_then(|(p, s)| Some((p.parse::<u8>().ok()?, s.parse::<u8>().ok()?)));

                TypeFamily::Decimal(params)
            }
            TypeIdentifier::Boolean => TypeFamily::Boolean,
            TypeIdentifier::Enum(_) => TypeFamily::Text(Some(TypeDataLength::Constant(8000))),
            TypeIdentifier::UUID => TypeFamily::Uuid,
            TypeIdentifier::Json => TypeFamily::Text(Some(TypeDataLength::Maximum)),
            TypeIdentifier::Xml => TypeFamily::Text(Some(TypeDataLength::Maximum)),
            TypeIdentifier::DateTime => TypeFamily::DateTime,
            TypeIdentifier::Bytes => TypeFamily::Text(parse_scalar_length(self)),
            TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
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
        PrismaValue::Null => Value::Int32(None), // Can't tell which type the null is supposed to be.
        PrismaValue::Object(_) => unimplemented!(),
    }
}

fn parse_scalar_length(sf: &ScalarField) -> Option<TypeDataLength> {
    sf.native_type
        .as_ref()
        .and_then(|nt| nt.args.first())
        .and_then(|len| match len.to_lowercase().as_str() {
            "max" => Some(TypeDataLength::Maximum),
            num => num.parse().map(TypeDataLength::Constant).ok(),
        })
}
