use std::unimplemented;

use crate::{IntoBson, MongoError};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{TimeZone, Utc};
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use prisma_models::PrismaValue;

impl IntoBson for PrismaValue {
    fn into_bson(self) -> crate::Result<Bson> {
        dbg!(&self);

        match self {
            PrismaValue::String(s) => Ok(Bson::String(s)),
            PrismaValue::Boolean(b) => Ok(Bson::Boolean(b)),
            PrismaValue::Enum(_) => Err(MongoError::Unsupported("Enums".to_owned())),
            PrismaValue::Int(i) => Ok(Bson::Int64(i)),
            PrismaValue::Uuid(u) => Ok(Bson::String(u.to_string())),
            PrismaValue::List(list) => Ok(Bson::Array(
                list.into_iter()
                    .map(|e| e.into_bson())
                    .collect::<crate::Result<Vec<_>>>()?,
            )),
            PrismaValue::Json(_) => unimplemented!("Figure out JSON => BSON conversion."),
            PrismaValue::Xml(_) => Err(MongoError::Unsupported("Mongo doesn't support enums.".to_owned())),
            PrismaValue::Null => Ok(Bson::Null),
            PrismaValue::DateTime(dt) => Ok(Bson::DateTime(dt.with_timezone(&Utc))),
            PrismaValue::Float(_) => unimplemented!("Figure out decimal to bigdecimal crate conversion."),
            PrismaValue::BigInt(i) => Ok(Bson::Int64(i)),
            PrismaValue::Bytes(b) => Ok(Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes: b,
            })),
        }
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
