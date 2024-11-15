use crate::context::Context;
use chrono::Utc;
use prisma_value::PrismaValue;
use quaint::{
    ast::{EnumName, Value, ValueType},
    prelude::{EnumVariant, TypeDataLength, TypeFamily},
};
use query_structure::{ScalarField, TypeIdentifier};

pub(crate) trait ScalarFieldExt {
    fn value<'a>(&self, pv: PrismaValue, ctx: &Context<'_>) -> Value<'a>;
    fn type_family(&self) -> TypeFamily;
}

impl ScalarFieldExt for ScalarField {
    fn value<'a>(&self, pv: PrismaValue, ctx: &Context<'_>) -> Value<'a> {
        let value = match (pv, self.type_identifier()) {
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

                Value::enum_variant_with_name(e, EnumName::new(enum_name, schema_name))
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

                Value::enum_array_with_name(variants, EnumName::new(enum_name, schema_name))
            }
            (PrismaValue::Enum(e), _) => e.into(),
            (PrismaValue::Int(i), _) => i.into(),
            (PrismaValue::BigInt(i), _) => i.into(),
            (PrismaValue::Uuid(u), _) => u.to_string().into(),
            (PrismaValue::List(l), _) => Value::array(l.into_iter().map(|x| self.value(x, ctx))),
            (PrismaValue::Json(s), _) => Value::json(serde_json::from_str::<serde_json::Value>(&s).unwrap()),
            (PrismaValue::Bytes(b), _) => Value::bytes(b),
            (PrismaValue::Object(_), _) => unimplemented!(),
            (PrismaValue::Null, ident) => match ident {
                TypeIdentifier::String => Value::null_text(),
                TypeIdentifier::Float => Value::null_numeric(),
                TypeIdentifier::Decimal => Value::null_numeric(),
                TypeIdentifier::Boolean => Value::null_boolean(),
                TypeIdentifier::Enum(enum_id) => {
                    let enum_walker = self.dm.clone().zip(enum_id);
                    let enum_name = enum_walker.db_name().to_owned();
                    let schema_name = enum_walker
                        .schema_name()
                        .map(ToOwned::to_owned)
                        .or(Some(ctx.schema_name().to_owned()));

                    ValueType::Enum(None, Some(EnumName::new(enum_name, schema_name))).into_value()
                }
                TypeIdentifier::Json => Value::null_json(),
                TypeIdentifier::DateTime => Value::null_datetime(),
                TypeIdentifier::UUID => Value::null_uuid(),
                TypeIdentifier::Int => Value::null_int32(),
                TypeIdentifier::BigInt => Value::null_int64(),
                TypeIdentifier::Bytes => Value::null_bytes(),
                TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
            },
        };

        let nt_col_type = self.native_type().map(|nt| (nt.name(), parse_scalar_length(self)));

        value.with_native_column_type(nt_col_type)
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
        PrismaValue::List(l) => Value::array(l.into_iter().map(convert_lossy)),
        PrismaValue::Json(s) => Value::json(serde_json::from_str(&s).unwrap()),
        PrismaValue::Bytes(b) => Value::bytes(b),
        PrismaValue::Null => Value::null_int32(), // Can't tell which type the null is supposed to be.
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
