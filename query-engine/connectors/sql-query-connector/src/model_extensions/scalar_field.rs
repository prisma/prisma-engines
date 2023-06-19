use chrono::Utc;
use prisma_models::{ScalarField, TypeIdentifier};
use prisma_value::PrismaValue;
use quaint::{
    ast::Value,
    prelude::{BytesTypeFamily, TextTypeFamily, TypeDataLength, TypeFamily},
};

pub trait ScalarFieldExt {
    fn value<'a>(&self, pv: PrismaValue) -> Value<'a>;
    fn type_family(&self) -> TypeFamily;
}

impl ScalarFieldExt for ScalarField {
    fn value<'a>(&self, pv: PrismaValue) -> Value<'a> {
        match (pv, self.type_identifier()) {
            (PrismaValue::String(s), _) => s.into(),
            (PrismaValue::Float(f), _) => f.into(),
            (PrismaValue::Boolean(b), _) => b.into(),
            (PrismaValue::DateTime(d), _) => d.with_timezone(&Utc).into(),
            (PrismaValue::Enum(e), _) => e.into(),
            (PrismaValue::Int(i), _) => i.into(),
            (PrismaValue::BigInt(i), _) => i.into(),
            (PrismaValue::Uuid(u), _) => u.to_string().into(),
            (PrismaValue::List(l), _) => Value::Array(Some(l.into_iter().map(|x| self.value(x)).collect())),
            (PrismaValue::Json(s), _) => Value::Json(Some(serde_json::from_str::<serde_json::Value>(&s).unwrap())),
            (PrismaValue::Bytes(b), _) => Value::Bytes(Some(b.into())),
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
                TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
            },
        }
    }

    fn type_family(&self) -> TypeFamily {
        let nt_name = self.native_type().map(|nt| nt.name());

        match (self.type_identifier(), nt_name) {
            // A specific XML type is required on SQL Server for INSERT OUTPUT.
            // As we use a temporary table to store the inserted values and then select them afterward,
            // the column type in that temporary table cannot be of type VARCHAR without additional casting.
            (TypeIdentifier::String, Some("Xml")) => TypeFamily::Text(None, Some(TextTypeFamily::Xml)),
            (TypeIdentifier::String, _) => TypeFamily::Text(parse_scalar_length(self), None),
            (TypeIdentifier::Int, _) => TypeFamily::Int,
            (TypeIdentifier::BigInt, _) => TypeFamily::Int,
            (TypeIdentifier::Float, _) => TypeFamily::Double,
            (TypeIdentifier::Decimal, _) => {
                let params = self
                    .native_type()
                    .map(|nt| nt.args().into_iter())
                    .and_then(|mut args| Some((args.next()?, args.next()?)))
                    .and_then(|(p, s)| Some((p.parse::<u8>().ok()?, s.parse::<u8>().ok()?)));

                TypeFamily::Decimal(params)
            }
            (TypeIdentifier::Boolean, _) => TypeFamily::Boolean,
            (TypeIdentifier::Enum(_), _) => TypeFamily::Text(Some(TypeDataLength::Constant(8000)), None),
            (TypeIdentifier::UUID, _) => TypeFamily::Uuid,
            (TypeIdentifier::Json, _) => TypeFamily::Text(Some(TypeDataLength::Maximum), None),
            (TypeIdentifier::DateTime, _) => TypeFamily::DateTime,
            // A specific Image type is required on SQL Server for INSERT OUTPUT.
            // As we use a temporary table to store the inserted values and then select them afterward,
            // the column type in that temporary table cannot be of type Bytes without additional casting.
            (TypeIdentifier::Bytes, Some("Image")) => TypeFamily::Bytes(None, Some(BytesTypeFamily::Image)),
            (TypeIdentifier::Bytes, _) => TypeFamily::Text(parse_scalar_length(self), None),
            (TypeIdentifier::Unsupported, _) => unreachable!("No unsupported field should reach that path"),
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
        PrismaValue::Int(i) => i.into(),
        PrismaValue::BigInt(i) => i.into(),
        PrismaValue::Uuid(u) => u.to_string().into(),
        PrismaValue::List(l) => Value::Array(Some(l.into_iter().map(convert_lossy).collect())),
        PrismaValue::Json(s) => Value::Json(serde_json::from_str(&s).unwrap()),
        PrismaValue::Bytes(b) => Value::Bytes(Some(b.into())),
        PrismaValue::Null => Value::Int32(None), // Can't tell which type the null is supposed to be.
        PrismaValue::Object(_) => unimplemented!(),
    }
}

fn parse_scalar_length(sf: &ScalarField) -> Option<TypeDataLength> {
    sf.native_type()
        .and_then(|nt| nt.args().into_iter().next())
        .and_then(|len| match len.to_lowercase().as_str() {
            "max" => Some(TypeDataLength::Maximum),
            num => num.parse().map(TypeDataLength::Constant).ok(),
        })
}
