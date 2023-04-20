use crate::{
    filter::FilterPrefix,
    output_meta::{CompositeOutputMeta, OutputMeta, ScalarOutputMeta},
    IntoBson, MongoError,
};
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{TimeZone, Utc};
use itertools::Itertools;
use mongodb::bson::{oid::ObjectId, spec::BinarySubtype, Binary, Bson, Document, Timestamp};
use prisma_models::{
    CompositeFieldRef, Field, PrismaValue, RelationFieldRef, ScalarFieldRef, SelectedField, TypeIdentifier,
};
use psl::builtin_connectors::MongoDbType;
use serde_json::Value;
use std::{convert::TryFrom, fmt::Display};

/// Transforms a `PrismaValue` of a specific selected field into the BSON mapping as prescribed by
/// the native types or as defined by the default `TypeIdentifier` to BSON mapping.
impl IntoBson for (&SelectedField, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        let (selection, value) = self;

        match selection {
            SelectedField::Scalar(sf) => (sf, value).into_bson(),
            SelectedField::Composite(_) => todo!(), // [Composites] todo
        }
    }
}

impl IntoBson for (&Field, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        let (selection, value) = self;

        match selection {
            Field::Scalar(sf) => (sf, value).into_bson(),
            Field::Composite(cf) => (cf, value).into_bson(),
            Field::Relation(_) => unreachable!("Relation fields should never hit the BSON conversion logic."),
        }
    }
}

impl IntoBson for (&CompositeFieldRef, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        let (cf, value) = self;

        match value {
            PrismaValue::Null => Ok(Bson::Null),
            PrismaValue::Object(pairs) if cf.is_list() => Ok(Bson::Array(vec![convert_composite_object(cf, pairs)?])),
            PrismaValue::Object(pairs) => convert_composite_object(cf, pairs),

            PrismaValue::List(values) => Ok(Bson::Array(
                values
                    .into_iter()
                    .map(|val| {
                        if let PrismaValue::Object(pairs) = val {
                            convert_composite_object(cf, pairs)
                        } else {
                            unreachable!("Composite lists must be objects")
                        }
                    })
                    .collect::<crate::Result<Vec<_>>>()?,
            )),

            _ => unreachable!("{}", value),
        }
    }
}

fn convert_composite_object(cf: &CompositeFieldRef, pairs: Vec<(String, PrismaValue)>) -> crate::Result<Bson> {
    let mut doc = Document::new();

    for (field, value) in pairs {
        let composite_type = cf.typ();
        let field = composite_type
            .find_field(&field) // Todo: This is assuming a lot by only checking the prisma names, not DB names.
            .expect("Writing unavailable composite field.");

        let converted = (&field, value).into_bson()?;

        doc.insert(field.db_name(), converted);
    }

    Ok(Bson::Document(doc))
}

impl IntoBson for (&FilterPrefix, &ScalarFieldRef) {
    fn into_bson(self) -> crate::Result<Bson> {
        let (prefix, sf) = self;

        Ok(Bson::String(prefix.render_with(sf.db_name().to_string())))
    }
}

impl IntoBson for (&FilterPrefix, &CompositeFieldRef) {
    fn into_bson(self) -> crate::Result<Bson> {
        let (prefix, cf) = self;

        Ok(Bson::String(prefix.render_with(cf.db_name().to_string())))
    }
}

impl IntoBson for (&FilterPrefix, &RelationFieldRef) {
    fn into_bson(self) -> crate::Result<Bson> {
        let (prefix, rf) = self;

        Ok(Bson::String(prefix.render_with(rf.relation().name())))
    }
}

impl IntoBson for (&ScalarFieldRef, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        let (sf, value) = self;

        let nt = sf.native_type();
        let mongo_type: Option<MongoDbType> = nt.map(|nt| nt.deserialize_native_type::<MongoDbType>().to_owned());

        // If we have a native type, use that one as source of truth for mapping, else use the type ident for defaults.
        match (mongo_type, &sf.type_identifier(), value) {
            // We assume this is always valid if it arrives here.
            (_, _, PrismaValue::Null) => Ok(Bson::Null),
            (Some(mt), _, value) => (&mt, value).into_bson(),
            (_, field_type, value) => (field_type, value).into_bson(),
        }
    }
}

/// Conversion using an explicit native type.
impl IntoBson for (&MongoDbType, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        Ok(match self {
            // ObjectId
            (MongoDbType::ObjectId, PrismaValue::String(s)) => Bson::ObjectId(ObjectId::parse_str(&s)?),
            (MongoDbType::ObjectId, PrismaValue::Bytes(b)) => {
                if b.len() != 12 {
                    return Err(MongoError::MalformedObjectId(format!(
                        "ObjectIDs require exactly 12 bytes, got: {}",
                        b.len()
                    )));
                }

                let mut bytes: [u8; 12] = [0x0; 12];
                bytes.iter_mut().set_from(b.into_iter());

                Bson::ObjectId(ObjectId::from_bytes(bytes))
            }

            // String
            (MongoDbType::String, PrismaValue::String(s)) => Bson::String(s),
            (MongoDbType::String, PrismaValue::Uuid(u)) => Bson::String(u.to_string()),

            // Double
            (MongoDbType::Double, PrismaValue::Int(i)) => Bson::Double(i as f64),
            (MongoDbType::Double, PrismaValue::Float(f)) => Bson::Double(f.to_f64().convert(expl::MONGO_DOUBLE)?),
            (MongoDbType::Double, PrismaValue::BigInt(b)) => Bson::Double(b.to_f64().convert(expl::MONGO_DOUBLE)?),

            // Int
            (MongoDbType::Int, PrismaValue::Int(b)) => Bson::Int32(b as i32),
            (MongoDbType::Int, PrismaValue::BigInt(b)) => Bson::Int32(b as i32),
            (MongoDbType::Int, PrismaValue::Float(b)) => Bson::Int32(b.to_i32().convert(expl::MONGO_I32)?),

            // Long
            (MongoDbType::Long, PrismaValue::Int(b)) => Bson::Int64(b),
            (MongoDbType::Long, PrismaValue::BigInt(b)) => Bson::Int64(b),
            (MongoDbType::Long, PrismaValue::Float(d)) => Bson::Int64(d.to_i64().convert(expl::MONGO_I64)?),

            // Array
            (typ, PrismaValue::List(vals)) => Bson::Array(
                vals.into_iter()
                    .map(|val| (typ, val).into_bson())
                    .collect::<crate::Result<Vec<_>>>()?,
            ),

            // BinData
            (MongoDbType::BinData, PrismaValue::Bytes(bytes)) => Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes,
            }),

            // Bool
            (MongoDbType::Bool, PrismaValue::Boolean(b)) => Bson::Boolean(b),

            // Date / Timestamp
            (MongoDbType::Date, PrismaValue::DateTime(dt)) => Bson::DateTime(dt.into()),
            (MongoDbType::Timestamp, PrismaValue::DateTime(dt)) => Bson::Timestamp(Timestamp {
                // We might not want to offer timestamp as a type, it's internal and mapping is weird.
                time: dt.timestamp() as u32,
                increment: 0,
            }),

            // Unhandled conversions
            (mdb_type, p_val) => {
                return Err(MongoError::ConversionError {
                    from: format!("{p_val:?}"),
                    to: format!("{mdb_type:?}"),
                })
            }
        })
    }
}

/// Convert the `PrismaValue` into Bson using the type hint given by `TypeIdentifier`.
impl IntoBson for (&TypeIdentifier, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        Ok(match self {
            // String & UUID
            (TypeIdentifier::String, PrismaValue::String(s)) => Bson::String(s),
            (TypeIdentifier::String, PrismaValue::Uuid(s)) => Bson::String(s.to_string()),
            (TypeIdentifier::UUID, PrismaValue::Uuid(s)) => Bson::String(s.to_string()),
            (TypeIdentifier::UUID, PrismaValue::String(s)) => Bson::String(s),

            // Enums
            (TypeIdentifier::Enum(_), PrismaValue::String(s)) => Bson::String(s),
            (TypeIdentifier::Enum(_), PrismaValue::Enum(s)) => Bson::String(s),

            // Bool
            (TypeIdentifier::Boolean, PrismaValue::Boolean(b)) => Bson::Boolean(b),

            // DateTime
            (TypeIdentifier::DateTime, PrismaValue::DateTime(dt)) => Bson::DateTime(dt.into()),

            // Int
            (TypeIdentifier::Int, PrismaValue::Int(i)) => Bson::Int64(i),
            (TypeIdentifier::Int, PrismaValue::BigInt(i)) => Bson::Int64(i),
            (TypeIdentifier::Int, PrismaValue::Float(dec)) => Bson::Int64(dec.to_i64().convert(expl::MONGO_I64)?),

            // BigInt
            (TypeIdentifier::BigInt, PrismaValue::BigInt(i)) => Bson::Int64(i),
            (TypeIdentifier::BigInt, PrismaValue::Int(i)) => Bson::Int64(i),
            (TypeIdentifier::BigInt, PrismaValue::Float(dec)) => Bson::Int64(dec.to_i64().convert(expl::MONGO_I64)?),

            // Float
            (TypeIdentifier::Float, PrismaValue::Float(dec)) => {
                // We don't have native support for float numbers (yet)
                // so we need to do this, see https://docs.rs/bigdecimal/latest/bigdecimal/index.html
                let dec_str = dec.to_string();
                let f64_val = dec_str.parse::<f64>().ok();
                let converted = f64_val.convert(expl::MONGO_DOUBLE)?;

                Bson::Double(converted)
            }
            (TypeIdentifier::Float, PrismaValue::Int(i)) => Bson::Double(i.to_f64().convert(expl::MONGO_DOUBLE)?),
            (TypeIdentifier::Float, PrismaValue::BigInt(i)) => Bson::Double(i.to_f64().convert(expl::MONGO_DOUBLE)?),

            // Decimal (todo properly when the driver supports dec128)
            (TypeIdentifier::Decimal, PrismaValue::Float(dec)) => {
                Bson::Double(dec.to_f64().convert(expl::MONGO_DOUBLE)?)
            }
            (TypeIdentifier::Decimal, PrismaValue::Int(i)) => Bson::Double(i.to_f64().convert(expl::MONGO_DOUBLE)?),
            (TypeIdentifier::Decimal, PrismaValue::BigInt(i)) => Bson::Double(i.to_f64().convert(expl::MONGO_DOUBLE)?),

            // Bytes
            (TypeIdentifier::Bytes, PrismaValue::Bytes(bytes)) => Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes,
            }),

            // Json
            (TypeIdentifier::Json, PrismaValue::Json(json)) => {
                let val: Value = serde_json::from_str(&json)?;
                Bson::try_from(val).map_err(|_| MongoError::ConversionError {
                    from: "Stringified JSON".to_owned(),
                    to: "Mongo BSON (extJSON)".to_owned(),
                })?
            }

            // List values
            (typ, PrismaValue::List(vals)) => Bson::Array(
                vals.into_iter()
                    .map(|val| (typ, val).into_bson())
                    .collect::<crate::Result<Vec<_>>>()?,
            ),

            // Unhandled mappings
            (TypeIdentifier::Xml, _) => return Err(MongoError::Unsupported("Mongo doesn't support XML.".to_owned())),
            (TypeIdentifier::Unsupported, _) => unreachable!("Unsupported types should never hit the connector."),

            (ident, val) => {
                return Err(MongoError::Unsupported(format!(
                    "Unhandled and unsupported value mapping for MongoDB: {val} as {ident:?}.",
                )))
            }
        })
    }
}

// Parsing of values coming from MongoDB back to the connector / core.
pub fn value_from_bson(bson: Bson, meta: &OutputMeta) -> crate::Result<PrismaValue> {
    match meta {
        OutputMeta::Scalar(scalar_meta) => read_scalar_value(bson, scalar_meta),
        OutputMeta::Composite(composite_meta) => read_composite_value(bson, composite_meta),
    }
}

fn read_scalar_value(bson: Bson, meta: &ScalarOutputMeta) -> crate::Result<PrismaValue> {
    let val = match (&meta.ident, bson) {
        // We expect a list to be returned.
        (type_identifier, bson) if meta.list => match bson {
            Bson::Null => PrismaValue::List(Vec::new()),

            Bson::Array(list) => PrismaValue::List(
                list.into_iter()
                    .map(|list_val| value_from_bson(list_val, &meta.strip_list().into()))
                    .collect::<crate::Result<Vec<_>>>()?,
            ),

            _ => {
                return Err(MongoError::ConversionError {
                    from: format!("{bson}"),
                    to: format!("List of {type_identifier:?}"),
                });
            }
        },

        // Null catch-all.
        (_, Bson::Null) => {
            if let Some(ref dv) = meta.default {
                dv.clone()
            } else {
                PrismaValue::Null
            }
        }

        // String + UUID + Enum
        (TypeIdentifier::String, Bson::String(s)) => PrismaValue::String(s),
        (TypeIdentifier::String, Bson::ObjectId(oid)) => PrismaValue::String(oid.to_string()),
        (TypeIdentifier::UUID, Bson::String(s)) => PrismaValue::Uuid(uuid::Uuid::parse_str(&s)?),
        (TypeIdentifier::Enum(_), Bson::String(s)) => PrismaValue::Enum(s),

        // Bool
        (TypeIdentifier::Boolean, Bson::Boolean(b)) => PrismaValue::Boolean(b),

        // Int
        (TypeIdentifier::Int, Bson::Int64(i)) => PrismaValue::Int(i),
        (TypeIdentifier::Int, Bson::Int32(i)) => PrismaValue::Int(i as i64),
        (TypeIdentifier::Int, Bson::Double(i)) => PrismaValue::Int(i as i64),
        (TypeIdentifier::Int, Bson::Boolean(bool)) => PrismaValue::Int(bool as i64),

        // BigInt
        (TypeIdentifier::BigInt, Bson::Int64(i)) => PrismaValue::BigInt(i),
        (TypeIdentifier::BigInt, Bson::Int32(i)) => PrismaValue::BigInt(i as i64),
        (TypeIdentifier::BigInt, Bson::Boolean(bool)) => PrismaValue::BigInt(bool as i64),

        // Floats
        (TypeIdentifier::Float, Bson::Double(f)) => {
            PrismaValue::Float(BigDecimal::from_f64(f).convert(expl::PRISMA_FLOAT)?.normalized())
        }
        (TypeIdentifier::Float, Bson::Int32(i)) => {
            PrismaValue::Float(BigDecimal::from_i64(i as i64).convert(expl::PRISMA_FLOAT)?.normalized())
        }
        (TypeIdentifier::Float, Bson::Int64(i)) => {
            PrismaValue::Float(BigDecimal::from_i64(i).convert(expl::PRISMA_FLOAT)?.normalized())
        }

        // Decimals
        (TypeIdentifier::Decimal, Bson::Double(f)) => {
            PrismaValue::Float(BigDecimal::from_f64(f).convert(expl::PRISMA_FLOAT)?.normalized())
        }
        (TypeIdentifier::Decimal, Bson::Int32(i)) => {
            PrismaValue::Float(BigDecimal::from_i64(i as i64).convert(expl::PRISMA_FLOAT)?.normalized())
        }
        (TypeIdentifier::Decimal, Bson::Int64(i)) => {
            PrismaValue::Float(BigDecimal::from_i64(i).convert(expl::PRISMA_FLOAT)?.normalized())
        }

        // DateTime
        (TypeIdentifier::DateTime, Bson::DateTime(dt)) => PrismaValue::DateTime(dt.to_chrono().into()),
        (TypeIdentifier::DateTime, Bson::Timestamp(ts)) => {
            PrismaValue::DateTime(Utc.timestamp(ts.time as i64, 0).into())
        }

        // Bytes
        (TypeIdentifier::Bytes, Bson::Binary(bin)) => PrismaValue::Bytes(bin.bytes),
        (TypeIdentifier::Bytes, Bson::ObjectId(oid)) => PrismaValue::Bytes(oid.bytes().to_vec()),

        // Json
        (TypeIdentifier::Json, bson) => PrismaValue::Json(serde_json::to_string(&bson.into_relaxed_extjson())?),

        (ident, bson) => {
            return Err(MongoError::ConversionError {
                from: bson.to_string(),
                to: format!("{ident:?}"),
            })
        }
    };

    Ok(val)
}

fn read_composite_value(bson: Bson, meta: &CompositeOutputMeta) -> crate::Result<PrismaValue> {
    let val = if meta.list {
        match bson {
            // Coerce null to empty list (Prisma doesn't have nullable lists)
            Bson::Null => PrismaValue::List(Vec::new()),

            Bson::Array(list) => PrismaValue::List(
                list.into_iter()
                    .map(|list_val| value_from_bson(list_val, &meta.strip_list().into()))
                    .collect::<crate::Result<Vec<_>>>()?,
            ),

            _ => {
                return Err(MongoError::ConversionError {
                    from: format!("{bson}"),
                    to: "List".to_owned(),
                });
            }
        }
    } else {
        // Null catch-all.
        match bson {
            Bson::Null => PrismaValue::Null,
            Bson::Document(mut doc) => {
                let mut pairs = Vec::with_capacity(doc.len());

                // This approach ensures that missing fields are filled,
                // so that the serialization can decide if this is invalid or not.
                for (field, meta) in meta.inner.iter() {
                    match (doc.remove(field), meta) {
                        (Some(value), _) => {
                            let value = value_from_bson(value, meta)?;
                            pairs.push((field.clone(), value))
                        }
                        // Coerce missing scalar lists as empty lists
                        (None, OutputMeta::Composite(meta)) if meta.list => {
                            pairs.push((field.clone(), PrismaValue::List(Vec::new())))
                        }
                        // Coerce missing scalars with their default values
                        (None, OutputMeta::Scalar(meta)) if meta.default.is_some() => {
                            pairs.push((field.clone(), meta.default.clone().unwrap()))
                        }
                        // Fill missing fields without default values with nulls.
                        (None, _) => pairs.push((field.clone(), PrismaValue::Null)),
                    }
                }

                PrismaValue::Object(pairs)
            }
            bson => {
                return Err(MongoError::ConversionError {
                    from: format!("{bson:?}"),
                    to: "Document".to_owned(),
                })
            }
        }
    };

    Ok(val)
}

trait UnwrapConversion<T: Display> {
    fn convert(self, to_type_explanation: &str) -> crate::Result<T>;
}

impl<T> UnwrapConversion<T> for Option<T>
where
    T: Display,
{
    fn convert(self, to_type_explanation: &str) -> crate::Result<T> {
        match self {
            Some(i) => Ok(i),
            None => Err(MongoError::ConversionError {
                from: format_opt(self),
                to: to_type_explanation.to_owned(),
            }),
        }
    }
}

fn format_opt<T: Display>(opt: Option<T>) -> String {
    match opt {
        Some(t) => format!("{t}"),
        None => "None".to_owned(),
    }
}

/// Explanation constants for conversion errors.
mod expl {
    #![allow(dead_code)]

    pub const MONGO_DOUBLE: &str = "MongoDB Double (64bit)";
    pub const MONGO_I32: &str = "MongoDB Int (32 bit)";
    pub const MONGO_I64: &str = "MongoDB Int (64 bit)";

    pub const PRISMA_FLOAT: &str = "Prisma Float (BigDecimal)";
    pub const PRISMA_BIGINT: &str = "Prisma BigInt (64 bit)";
    pub const PRISMA_INT: &str = "Prisma Int (64 bit)";
}
