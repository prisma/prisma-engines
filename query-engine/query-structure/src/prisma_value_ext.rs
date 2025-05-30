use super::{PrismaValue, Type, TypeIdentifier};
use crate::DomainError;
use bigdecimal::ToPrimitive;
use prisma_value::{Placeholder, PrismaValueType};

pub(crate) trait PrismaValueExtensions {
    fn coerce(self, to_type: &Type) -> crate::Result<PrismaValue>;
}

impl PrismaValueExtensions for PrismaValue {
    // Todo this is not exhaustive for now.
    fn coerce(self, to_type: &Type) -> crate::Result<PrismaValue> {
        let coerced = match (self, to_type.id) {
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
            (PrismaValue::List(list), _) => PrismaValue::List(
                list.into_iter()
                    .map(|val| val.coerce(to_type))
                    .collect::<crate::Result<Vec<_>>>()?,
            ),

            (PrismaValue::Placeholder(Placeholder { name, r#type }), _) if r#type == to_type.to_prisma_type() => {
                PrismaValue::Placeholder(Placeholder { name, r#type })
            }

            (
                PrismaValue::Placeholder(Placeholder {
                    name,
                    r#type: PrismaValueType::Any,
                }),
                _,
            ) => PrismaValue::Placeholder(Placeholder {
                name,
                r#type: to_type.to_prisma_type(),
            }),

            // Invalid coercion
            (val, typ) => return Err(DomainError::ConversionFailure(format!("{val:?}"), format!("{typ:?}"))),
        };

        Ok(coerced)
    }
}
