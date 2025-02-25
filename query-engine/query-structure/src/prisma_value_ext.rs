use super::{PrismaValue, TypeIdentifier};
use crate::DomainError;
use bigdecimal::ToPrimitive;
use prisma_value::PlaceholderType;

pub(crate) trait PrismaValueExtensions {
    fn coerce(self, to_type: TypeIdentifier) -> crate::Result<PrismaValue>;
}

impl PrismaValueExtensions for PrismaValue {
    // Todo this is not exhaustive for now.
    fn coerce(self, to_type: TypeIdentifier) -> crate::Result<PrismaValue> {
        let coerced = match (self, to_type) {
            // Trivial cases
            (PrismaValue::Null, _) => PrismaValue::Null,
            (val @ PrismaValue::String(_), TypeIdentifier::String) => val,
            (val @ PrismaValue::Int(_), TypeIdentifier::Int) => val,
            (val @ PrismaValue::Float(_), TypeIdentifier::Float) => val,
            (val @ PrismaValue::Float(_), TypeIdentifier::Decimal) => val,
            (val @ PrismaValue::Boolean(_), TypeIdentifier::Boolean) => val,
            (val @ PrismaValue::DateTime(_), TypeIdentifier::DateTime) => val,
            (val @ PrismaValue::Enum(_), TypeIdentifier::Enum(_)) => val,
            (val @ PrismaValue::Uuid(_), TypeIdentifier::UUID) => val,
            (val @ PrismaValue::BigInt(_), TypeIdentifier::BigInt) => val,
            (val @ PrismaValue::Bytes(_), TypeIdentifier::Bytes) => val,
            (val @ PrismaValue::Json(_), TypeIdentifier::Json) => val,

            // Valid String coercions
            (PrismaValue::Int(i), TypeIdentifier::String) => PrismaValue::String(format!("{i}")),
            (PrismaValue::Float(f), TypeIdentifier::String) => PrismaValue::String(f.to_string()),
            (PrismaValue::Boolean(b), TypeIdentifier::String) => PrismaValue::String(format!("{b}")),
            (PrismaValue::Enum(e), TypeIdentifier::String) => PrismaValue::String(e),
            (PrismaValue::Uuid(u), TypeIdentifier::String) => PrismaValue::String(u.to_string()),

            // Valid Int coersions
            (PrismaValue::String(s), TypeIdentifier::Int) => match s.parse() {
                Ok(i) => PrismaValue::Int(i),
                Err(_) => return Err(DomainError::ConversionFailure(format!("{s:?}"), format!("{to_type:?}"))),
            },
            (PrismaValue::Float(f), TypeIdentifier::Int) => PrismaValue::Int(f.to_i64().unwrap()),
            (PrismaValue::BigInt(i), TypeIdentifier::Int) => PrismaValue::Int(i),

            // Valid BigInt coersions
            (PrismaValue::Int(i), TypeIdentifier::BigInt) => PrismaValue::BigInt(i),

            // Todo other coercions here

            // Lists
            (PrismaValue::List(list), typ) => PrismaValue::List(
                list.into_iter()
                    .map(|val| val.coerce(typ))
                    .collect::<crate::Result<Vec<_>>>()?,
            ),

            (PrismaValue::Placeholder { name, r#type }, typ) if r#type == typ.to_placeholder_type() => {
                PrismaValue::Placeholder { name, r#type }
            }

            (
                PrismaValue::Placeholder {
                    name,
                    r#type: PlaceholderType::Any,
                },
                typ,
            ) => PrismaValue::Placeholder {
                name,
                r#type: typ.to_placeholder_type(),
            },

            // Invalid coercion
            (val, typ) => return Err(DomainError::ConversionFailure(format!("{val:?}"), format!("{typ:?}"))),
        };

        Ok(coerced)
    }
}
