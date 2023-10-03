use crate::context::Context;
use chrono::Utc;
use prisma_models::{ScalarField, TypeIdentifier};
use prisma_value::PrismaValue;
use quaint::{
    ast::{EnumName, Value, ValueType},
    prelude::{EnumVariant, TypeDataLength, TypeFamily},
};

pub(crate) trait ScalarFieldExt {
    fn value<'a>(&self, pv: PrismaValue, ctx: &Context<'_>) -> Value<'a>;
    fn type_family(&self) -> TypeFamily;
}

impl ScalarFieldExt for ScalarField {
    fn value<'a>(&self, pv: PrismaValue, ctx: &Context<'_>) -> Value<'a> {
        match (pv, self.type_identifier()) {
            (PrismaValue::String(s), _) => s.into(),
            (PrismaValue::Float(f), _) => f.into(),
            (PrismaValue::Boolean(b), _) => b.into(),
            (PrismaValue::DateTime(d), _) => d.with_timezone(&Utc).into(),
            (PrismaValue::Enum(e), TypeIdentifier::Enum(enum_id)) => {
                let enum_walker = self.dm.clone().zip(enum_id);
                let enum_name = enum_walker.db_name().to_owned();
                let schema_name = enum_walker
                    .schema_name()
                    .map(ToOwned::to_owned)
                    .or(Some(ctx.schema_name().to_owned()));

                Value::enum_variant_with_name(e, enum_name, schema_name)
            }
            (PrismaValue::List(vals), TypeIdentifier::Enum(enum_id)) => {
                let enum_walker = self.dm.clone().zip(enum_id);
                let variants: Vec<_> = vals
                    .into_iter()
                    .map(|val| val.into_string().unwrap())
                    .map(EnumVariant::new)
                    .collect();

                let enum_name = enum_walker.db_name().to_owned();
                let schema_name = enum_walker
                    .schema_name()
                    .map(ToOwned::to_owned)
                    .or(Some(ctx.schema_name().to_owned()));

                ValueType::EnumArray(Some(variants), Some(EnumName::new(enum_name, schema_name))).into()
            }
            (PrismaValue::Enum(e), _) => e.into(),
            (PrismaValue::Int(i), _) => i.into(),
            (PrismaValue::BigInt(i), _) => i.into(),
            (PrismaValue::Uuid(u), _) => u.to_string().into(),
            (PrismaValue::List(l), _) => {
                ValueType::Array(Some(l.into_iter().map(|x| self.value(x, ctx)).collect())).into()
            }
            (PrismaValue::Json(s), _) => {
                ValueType::Json(Some(serde_json::from_str::<serde_json::Value>(&s).unwrap())).into()
            }
            (PrismaValue::Bytes(b), _) => ValueType::Bytes(Some(b.into())).into(),
            (PrismaValue::Object(_), _) => unimplemented!(),
            (PrismaValue::Null, ident) => match ident {
                TypeIdentifier::String => ValueType::Text(None).into(),
                TypeIdentifier::Float => ValueType::Numeric(None).into(),
                TypeIdentifier::Decimal => ValueType::Numeric(None).into(),
                TypeIdentifier::Boolean => ValueType::Boolean(None).into(),
                TypeIdentifier::Enum(enum_id) => {
                    let enum_walker = self.dm.clone().zip(enum_id);
                    let enum_name = enum_walker.db_name().to_owned();
                    let schema_name = enum_walker
                        .schema_name()
                        .map(ToOwned::to_owned)
                        .or(Some(ctx.schema_name().to_owned()));

                    ValueType::Enum(None, Some(EnumName::new(enum_name, schema_name))).into()
                }
                TypeIdentifier::Json => ValueType::Json(None).into(),
                TypeIdentifier::DateTime => ValueType::DateTime(None).into(),
                TypeIdentifier::UUID => ValueType::Uuid(None).into(),
                TypeIdentifier::Int => ValueType::Int32(None).into(),
                TypeIdentifier::BigInt => ValueType::Int64(None).into(),
                TypeIdentifier::Bytes => ValueType::Bytes(None).into(),
                TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
            },
        }
    }

    fn type_family(&self) -> TypeFamily {
        match self.type_identifier() {
            TypeIdentifier::String => TypeFamily::Text(parse_scalar_length(self)),
            TypeIdentifier::Int => TypeFamily::Int,
            TypeIdentifier::BigInt => TypeFamily::Int,
            TypeIdentifier::Float => TypeFamily::Double,
            TypeIdentifier::Decimal => {
                let params = self
                    .native_type()
                    .map(|nt| nt.args().into_iter())
                    .and_then(|mut args| Some((args.next()?, args.next()?)))
                    .and_then(|(p, s)| Some((p.parse::<u8>().ok()?, s.parse::<u8>().ok()?)));

                TypeFamily::Decimal(params)
            }
            TypeIdentifier::Boolean => TypeFamily::Boolean,
            TypeIdentifier::Enum(_) => TypeFamily::Text(Some(TypeDataLength::Constant(8000))),
            TypeIdentifier::UUID => TypeFamily::Uuid,
            TypeIdentifier::Json => TypeFamily::Text(Some(TypeDataLength::Maximum)),
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
        PrismaValue::Int(i) => i.into(),
        PrismaValue::BigInt(i) => i.into(),
        PrismaValue::Uuid(u) => u.to_string().into(),
        PrismaValue::List(l) => ValueType::Array(Some(l.into_iter().map(convert_lossy).collect())).into(),
        PrismaValue::Json(s) => ValueType::Json(serde_json::from_str(&s).unwrap()).into(),
        PrismaValue::Bytes(b) => ValueType::Bytes(Some(b.into())).into(),
        PrismaValue::Null => ValueType::Int32(None).into(), // Can't tell which type the null is supposed to be.
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
