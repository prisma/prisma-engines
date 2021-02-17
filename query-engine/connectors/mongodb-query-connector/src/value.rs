use std::unimplemented;

use crate::{IntoBson, MongoError};
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{TimeZone, Utc};
use mongodb::bson::{oid::ObjectId, spec::BinarySubtype, Binary, Bson, Timestamp};
use native_types::MongoDbType;
use prisma_models::{PrismaValue, ScalarFieldRef, TypeIdentifier};

// Straight value mapping without using any type information.
// impl IntoBson for PrismaValue {
//     fn into_bson(self) -> crate::Result<Bson> {
//         match self {
//             PrismaValue::String(s) => Ok(Bson::String(s)),
//             PrismaValue::Boolean(b) => Ok(Bson::Boolean(b)),
//             PrismaValue::Enum(_) => Err(MongoError::Unsupported("Enums".to_owned())),
//             PrismaValue::Int(i) => Ok(Bson::Int64(i)),
//             PrismaValue::Uuid(u) => Ok(Bson::String(u.to_string())),
//             PrismaValue::List(list) => Ok(Bson::Array(
//                 list.into_iter()
//                     .map(|e| e.into_bson())
//                     .collect::<crate::Result<Vec<_>>>()?,
//             )),
//             PrismaValue::Json(_) => unimplemented!("Figure out JSON => BSON conversion."),
//             PrismaValue::Xml(_) => Err(MongoError::Unsupported("Mongo doesn't support XML.".to_owned())),
//             PrismaValue::Null => Ok(Bson::Null),
//             PrismaValue::DateTime(dt) => Ok(Bson::DateTime(dt.with_timezone(&Utc))),
//             PrismaValue::Float(dec) => Ok(Bson::Double(dec.to_f64().unwrap())),
//             PrismaValue::BigInt(i) => Ok(Bson::Int64(i)),
//             PrismaValue::Bytes(b) => Ok(Bson::Binary(Binary {
//                 subtype: BinarySubtype::Generic,
//                 bytes: b,
//             })),
//         }
//     }
// }

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

// PrismaValue::String(s) if is_object_id => {
//     Ok(Bson::ObjectId(ObjectId::with_string(&s).map_err(|_err| {
//         MongoError::ConversionError {
//             from: s,
//             to: "ObjectID".to_owned(),
//         }
//     })?))
// }
// val => val.into_bson(),

impl IntoBson for (MongoDbType, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        Ok(match self {
            // ObjectId
            (MongoDbType::ObjectId, PrismaValue::String(s)) => Bson::ObjectId(ObjectId::with_string(&s)?),

            // String
            (MongoDbType::String, PrismaValue::String(s)) => Bson::String(s),
            (MongoDbType::String, PrismaValue::Uuid(u)) => Bson::String(u.to_string()),

            // Double
            (MongoDbType::Double, PrismaValue::Int(i)) => Bson::Double(i as f64),
            (MongoDbType::Double, PrismaValue::Float(f)) => {
                Bson::Double(f.to_f64().expect("Prisma Float can't be represented as Mongo Double."))
            }
            (MongoDbType::Double, PrismaValue::BigInt(b)) => {
                Bson::Double(b.to_f64().expect("Prisma BigInt can't be represented as Mongo Double."))
            }

            // Decimal
            (MongoDbType::Decimal, _) => unimplemented!("Mongo decimals."),

            // Int
            (MongoDbType::Int, PrismaValue::Int(b)) => Bson::Int32(b as i32),
            (MongoDbType::Int, PrismaValue::BigInt(b)) => Bson::Int32(b as i32),
            (MongoDbType::Int, PrismaValue::Float(b)) => Bson::Int32(
                b.to_i32()
                    .expect("Prisma Float can't be represented as Mongo Int (32 bit)"),
            ),

            // Long
            (MongoDbType::Long, PrismaValue::Int(b)) => Bson::Int64(b),
            (MongoDbType::Long, PrismaValue::BigInt(b)) => Bson::Int64(b),
            (MongoDbType::Long, PrismaValue::Float(d)) => Bson::Int64(
                d.to_i64()
                    .expect("Prisma Float can't be represented as Mongo Long (64 bit)"),
            ),

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

            // Todo
            // MongoDbType::MinKey
            // MongoDbType::MaxKey
            // MongoDbType::Object
            mapping => todo!("{:?}", mapping),
        })
    }
}

impl IntoBson for (&TypeIdentifier, PrismaValue) {
    fn into_bson(self) -> crate::Result<Bson> {
        todo!()
    }
}

pub(crate) fn value_from_bson(bson: Bson) -> crate::Result<PrismaValue> {
    match bson {
        Bson::Double(d) => match BigDecimal::from_f64(d) {
            Some(decimal) => Ok(PrismaValue::Float(decimal)),
            None => Err(MongoError::ConversionError {
                from: format!("{}", d),
                to: "Decimal".to_owned(),
            }),
        },
        Bson::Array(list) => Ok(PrismaValue::List(
            list.into_iter()
                .map(|bson| value_from_bson(bson))
                .collect::<crate::Result<Vec<_>>>()?,
        )),
        Bson::String(s) => Ok(PrismaValue::String(s)),
        Bson::Document(_) => unimplemented!("Figure out BSON => JSON conversion."),
        Bson::Boolean(b) => Ok(PrismaValue::Boolean(b)),
        Bson::Null => Ok(PrismaValue::Null),
        Bson::Int32(i) => Ok(PrismaValue::Int(i as i64)),
        Bson::Int64(i) => Ok(PrismaValue::Int(i)),
        Bson::DateTime(dt) => Ok(PrismaValue::DateTime(dt.into())),
        Bson::Timestamp(ts) => Ok(PrismaValue::DateTime(Utc.timestamp(ts.time as i64, 0).into())),
        Bson::Binary(bin) => Ok(PrismaValue::Bytes(bin.bytes)),
        Bson::ObjectId(oid) => Ok(PrismaValue::String(oid.to_hex())),
        Bson::Decimal128(_) => unimplemented!("Figure out decimal to bigdecimal crate conversion."),
        Bson::RegularExpression(_) => Err(MongoError::Unsupported("Regex Mongo type.".to_owned())),
        Bson::JavaScriptCode(_) => Err(MongoError::Unsupported("JS code Mongo type.".to_owned())),
        Bson::JavaScriptCodeWithScope(_) => Err(MongoError::Unsupported("JS code with scope Mongo type.".to_owned())),
        Bson::Symbol(_) => Err(MongoError::Unsupported("Symbol Mongo type.".to_owned())),
        Bson::Undefined => Err(MongoError::Unsupported("Undefined  Mongo type.".to_owned())),
        Bson::MaxKey => Err(MongoError::Unsupported("MaxKey Mongo type.".to_owned())),
        Bson::MinKey => Err(MongoError::Unsupported("MinKey Mongo type.".to_owned())),
        Bson::DbPointer(_) => Err(MongoError::Unsupported("DbPointer Mongo type.".to_owned())),
    }
}
