use crate::{output_meta::OutputMeta, IntoBson, MongoError};
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{TimeZone, Utc};
use itertools::Itertools;
use mongodb::bson::{oid::ObjectId, spec::BinarySubtype, Binary, Bson, Timestamp};
use native_types::MongoDbType;
use prisma_models::{PrismaValue, ScalarFieldRef, TypeIdentifier};
use serde_json::Value;
use std::{convert::TryFrom, fmt::Display};

/// Transforms a `PrismaValue` of a specific field into the BSON mapping as prescribed by the native types
/// or as defined by the default `TypeIdentifier` to BSON mapping.
impl IntoBson for (&ScalarFieldRef, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        let (field, value) = self;

        // This is _insanely_ inefficient, but we have no real choice with the current interface.
        let mongo_type: Option<MongoDbType> = field.native_type.as_ref().map(|nt| nt.deserialize_native_type());

        // If we have a native type, use that one as source of truth for mapping, else use the type ident for defaults.
        match (mongo_type, &field.type_identifier, value) {
            // We assume this is always valid if it arrives here.
            (_, _, PrismaValue::Null) => Ok(Bson::Null),
            (Some(mt), _, value) => (mt, value).into_bson(),
            (_, field_type, value) => (field_type, value).into_bson(),
        }
    }
}

/// Conversion using an explicit native type.
impl IntoBson for (MongoDbType, PrismaValue) {
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

            // Decimal
            (MongoDbType::Decimal, _) => unimplemented!("Mongo decimals."),

            // Int
            (MongoDbType::Int, PrismaValue::Int(b)) => Bson::Int32(b as i32),
            (MongoDbType::Int, PrismaValue::BigInt(b)) => Bson::Int32(b as i32),
            (MongoDbType::Int, PrismaValue::Float(b)) => Bson::Int32(b.to_i32().convert(expl::MONGO_I32)?),

            // Long
            (MongoDbType::Long, PrismaValue::Int(b)) => Bson::Int64(b),
            (MongoDbType::Long, PrismaValue::BigInt(b)) => Bson::Int64(b),
            (MongoDbType::Long, PrismaValue::Float(d)) => Bson::Int64(d.to_i64().convert(expl::MONGO_I64)?),

            // Array
            (MongoDbType::Array(inner), PrismaValue::List(vals)) => {
                let inner = *inner;
                Bson::Array(
                    vals.into_iter()
                        .map(|val| (inner.clone(), val).into_bson())
                        .collect::<crate::Result<Vec<_>>>()?,
                )
            }
            (MongoDbType::Array(inner), val) => Bson::Array(vec![(*inner, val).into_bson()?]),
            (typ, PrismaValue::List(vals)) => Bson::Array(
                vals.into_iter()
                    .map(|val| (typ.clone(), val).into_bson())
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
                    from: format!("{:?}", p_val),
                    to: format!("{:?}", mdb_type),
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
            (TypeIdentifier::Float, PrismaValue::Float(dec)) => Bson::Double(dec.to_f64().convert(expl::MONGO_DOUBLE)?),
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
                    "Unhandled and unsupported value mapping for MongoDB: {} as {:?}.",
                    val, ident,
                )))
            }
        })
    }
}

// Parsing of values coming from MongoDB back to the connector / core.
pub fn value_from_bson(bson: Bson, meta: &OutputMeta) -> crate::Result<PrismaValue> {
    let val = match (&meta.ident, bson) {
        // We expect a list to be returned.
        (type_identifier, bson) if meta.list => match bson {
            Bson::Null => PrismaValue::List(Vec::new()),

            Bson::Array(list) => PrismaValue::List(
                list.into_iter()
                    .map(|list_val| value_from_bson(list_val, &meta.strip_list()))
                    .collect::<crate::Result<Vec<_>>>()?,
            ),

            _ => {
                return Err(MongoError::ConversionError {
                    from: format!("{}", bson),
                    to: format!("List of {:?}", type_identifier),
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

        // BigInt
        (TypeIdentifier::BigInt, Bson::Int64(i)) => PrismaValue::BigInt(i),
        (TypeIdentifier::BigInt, Bson::Int32(i)) => PrismaValue::BigInt(i as i64),

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
            return Err(MongoError::UnhandledError(format!(
                "Converting BSON to type {:?}. Data: {:?}",
                ident, bson
            )))
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
        Some(t) => format!("{}", t),
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
