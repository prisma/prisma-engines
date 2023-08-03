use std::str::FromStr;

use chrono::Utc;
use geozero::{geojson::GeoJson, ToWkt};
use prisma_models::{ScalarField, TypeIdentifier};
use prisma_value::PrismaValue;
use quaint::{
    ast::GeometryValue,
    ast::Value,
    prelude::{TypeDataLength, TypeFamily},
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
            (PrismaValue::GeoJson(s), _) => {
                let geometry = GeometryValue {
                    wkt: GeoJson(&s).to_wkt().unwrap(),
                    srid: 4326,
                };
                match self.type_family() {
                    TypeFamily::Geography(_) => Value::Geography(Some(geometry)),
                    _ => Value::Geometry(Some(geometry)),
                }
            }
            (PrismaValue::Geometry(s), _) => {
                let geometry = GeometryValue::from_str(&s).unwrap();
                match self.type_family() {
                    TypeFamily::Geography(_) => Value::Geography(Some(geometry)),
                    _ => Value::Geometry(Some(geometry)),
                }
            }
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
                TypeIdentifier::Geometry(_) => match self.type_family() {
                    TypeFamily::Geography(_) => Value::Geography(None),
                    _ => Value::Geometry(None),
                },
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
            TypeIdentifier::Geometry(_) => {
                let type_info = self.native_type().map(|nt| {
                    let name = nt.name();
                    let srid = match nt.args().as_slice() {
                        [srid] => srid.parse::<i32>().ok(),
                        [_, srid] => srid.parse::<i32>().ok(),
                        _ => None,
                    };
                    (name, srid)
                });
                match type_info {
                    Some(("Geography", srid)) => TypeFamily::Geography(srid),
                    Some((_, srid)) => TypeFamily::Geometry(srid),
                    _ => TypeFamily::Geometry(None),
                }
            }
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
        PrismaValue::List(l) => Value::Array(Some(l.into_iter().map(convert_lossy).collect())),
        PrismaValue::Json(s) => Value::Json(serde_json::from_str(&s).unwrap()),
        PrismaValue::Bytes(b) => Value::Bytes(Some(b.into())),
        // TODO@geom: Fix this when we know how to cast GeoJSON to an appropriate DB value
        PrismaValue::GeoJson(s) => Value::Json(serde_json::from_str(&s).unwrap()),
        PrismaValue::Geometry(s) => Value::Geometry(Some(GeometryValue::from_str(&s).unwrap())),
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
