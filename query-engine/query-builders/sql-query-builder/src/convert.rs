use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveTime};
use prisma_value::{PrismaValue, PrismaValueType};
use quaint::ast::OpaqueType;
use query_builder::{ArgScalarType, ArgType, Arity, DynamicArgType};
use query_structure::{FieldArity, FieldTypeInformation, TypeIdentifier};

use crate::value::{GeneratorCall, Placeholder};

pub fn quaint_value_to_prisma_value(value: quaint::ValueType<'_>) -> PrismaValue {
    match value {
        quaint::ValueType::Int32(Some(i)) => PrismaValue::Int(i.into()),
        quaint::ValueType::Int32(None) => PrismaValue::Null,
        quaint::ValueType::Int64(Some(i)) => PrismaValue::BigInt(i),
        quaint::ValueType::Int64(None) => PrismaValue::Null,
        quaint::ValueType::Float(Some(f)) => PrismaValue::Float(
            BigDecimal::from_f32(f)
                .expect("float to decimal conversion should succeed")
                .normalized(),
        ),
        quaint::ValueType::Float(None) => PrismaValue::Null,
        quaint::ValueType::Double(Some(d)) => PrismaValue::Float(
            BigDecimal::from_f64(d)
                .expect("double to decimal conversion should succeed")
                .normalized(),
        ),
        quaint::ValueType::Double(None) => PrismaValue::Null,
        quaint::ValueType::Text(Some(s)) => PrismaValue::String(s.into_owned()),
        quaint::ValueType::Text(None) => PrismaValue::Null,
        quaint::ValueType::Enum(Some(e), _) => PrismaValue::Enum(e.into_owned()),
        quaint::ValueType::Enum(None, _) => PrismaValue::Null,
        quaint::ValueType::EnumArray(Some(es), _) => PrismaValue::List(
            es.into_iter()
                .map(|e| e.into_text())
                .map(|v| quaint_value_to_prisma_value(v.typed))
                .collect(),
        ),
        quaint::ValueType::EnumArray(None, _) => PrismaValue::Null,
        quaint::ValueType::Bytes(Some(b)) => PrismaValue::Bytes(b.into_owned()),
        quaint::ValueType::Bytes(None) => PrismaValue::Null,
        quaint::ValueType::Boolean(Some(b)) => PrismaValue::Boolean(b),
        quaint::ValueType::Boolean(None) => PrismaValue::Null,
        quaint::ValueType::Char(Some(c)) => PrismaValue::String(c.to_string()),
        quaint::ValueType::Char(None) => PrismaValue::Null,
        quaint::ValueType::Array(Some(a)) => {
            PrismaValue::List(a.into_iter().map(|v| quaint_value_to_prisma_value(v.typed)).collect())
        }
        quaint::ValueType::Array(None) => PrismaValue::Null,
        // We can't use PrismValue::Float with BigDecimal, because its serializer loses precision.
        quaint::ValueType::Numeric(Some(bd)) => PrismaValue::String(bd.to_string()),
        quaint::ValueType::Numeric(None) => PrismaValue::Null,
        quaint::ValueType::Json(Some(j)) => PrismaValue::Json(j.to_string()),
        quaint::ValueType::Json(None) => PrismaValue::Null,
        quaint::ValueType::Xml(Some(x)) => PrismaValue::String(x.into_owned()),
        quaint::ValueType::Xml(None) => PrismaValue::Null,
        quaint::ValueType::Uuid(Some(u)) => PrismaValue::Uuid(u),
        quaint::ValueType::Uuid(None) => PrismaValue::Null,
        quaint::ValueType::DateTime(Some(dt)) => PrismaValue::DateTime(dt.fixed_offset()),
        quaint::ValueType::DateTime(None) => PrismaValue::Null,
        quaint::ValueType::Date(Some(d)) => PrismaValue::DateTime(d.and_time(NaiveTime::MIN).and_utc().fixed_offset()),
        quaint::ValueType::Date(None) => PrismaValue::Null,
        quaint::ValueType::Time(Some(t)) => {
            PrismaValue::DateTime(DateTime::UNIX_EPOCH.date_naive().and_time(t).and_utc().fixed_offset())
        }
        quaint::ValueType::Time(None) => PrismaValue::Null,
        quaint::ValueType::Opaque(opaque) => {
            if let Some(placeholder) = opaque.downcast_ref::<Placeholder>() {
                PrismaValue::placeholder(placeholder.name().clone(), opaque_type_to_prisma_type(opaque.typ()))
            } else if let Some(call) = opaque.downcast_ref::<GeneratorCall>() {
                PrismaValue::GeneratorCall {
                    name: call.name().to_owned().into(),
                    args: call.args().to_vec(),
                    return_type: opaque_type_to_prisma_type(opaque.typ()),
                }
            } else {
                panic!("Received an unsupported opaque value")
            }
        }
    }
}

pub fn opaque_type_to_prisma_type(vt: &OpaqueType) -> PrismaValueType {
    match vt {
        OpaqueType::Unknown | OpaqueType::Tuple(_) => PrismaValueType::Any,
        OpaqueType::Int32 => PrismaValueType::Int,
        OpaqueType::Int64 => PrismaValueType::BigInt,
        OpaqueType::Float | OpaqueType::Double | OpaqueType::Numeric => PrismaValueType::Float,
        OpaqueType::Enum => PrismaValueType::Enum,
        OpaqueType::Text | OpaqueType::Xml | OpaqueType::Char => PrismaValueType::String,
        OpaqueType::Uuid => PrismaValueType::Uuid,
        OpaqueType::Bytes => PrismaValueType::Bytes,
        OpaqueType::Boolean => PrismaValueType::Boolean,
        OpaqueType::Array(inner) => PrismaValueType::List(opaque_type_to_prisma_type(inner).into()),
        OpaqueType::Json => PrismaValueType::Json,
        OpaqueType::Object => PrismaValueType::Object,
        OpaqueType::DateTime | OpaqueType::Date | OpaqueType::Time => PrismaValueType::DateTime,
    }
}

pub fn prisma_type_to_arg_type(pt: &PrismaValueType) -> ArgType {
    let scalar_type = match pt {
        PrismaValueType::Any => ArgScalarType::Unknown,
        PrismaValueType::String => ArgScalarType::String,
        PrismaValueType::Uuid => ArgScalarType::Uuid,
        PrismaValueType::Int => ArgScalarType::Int,
        PrismaValueType::BigInt => ArgScalarType::BigInt,
        PrismaValueType::Float => ArgScalarType::Float,
        PrismaValueType::Boolean => ArgScalarType::Boolean,
        PrismaValueType::DateTime => ArgScalarType::DateTime,
        PrismaValueType::Json | PrismaValueType::Object => ArgScalarType::Json,
        PrismaValueType::Bytes => ArgScalarType::Bytes,
        PrismaValueType::Enum => ArgScalarType::Enum,
        PrismaValueType::List(inner) => {
            let inner = prisma_type_to_arg_type(inner);
            assert_eq!(inner.arity, Arity::Scalar, "list element type must be a scalar type");
            return ArgType::new(Arity::List, inner.scalar_type, None);
        }
    };
    ArgType::new(Arity::Scalar, scalar_type, None)
}

pub fn quaint_value_to_arg_type(value: &quaint::Value<'_>) -> DynamicArgType {
    let native_type = value.native_column_type.as_deref().map(|nt| nt.to_owned());
    let scalar_type = match &value.typed {
        quaint::ValueType::Int32(_) => ArgScalarType::Int,
        quaint::ValueType::Int64(_) => ArgScalarType::BigInt,
        quaint::ValueType::Float(_) | quaint::ValueType::Double(_) => ArgScalarType::Float,
        quaint::ValueType::Numeric(_) => ArgScalarType::Decimal,
        quaint::ValueType::Enum(_, _) => ArgScalarType::Enum,
        quaint::ValueType::Text(_) | quaint::ValueType::Xml(_) | quaint::ValueType::Char(_) => ArgScalarType::String,
        quaint::ValueType::Uuid(_) => ArgScalarType::Uuid,
        quaint::ValueType::Bytes(_) => ArgScalarType::Bytes,
        quaint::ValueType::Boolean(_) => ArgScalarType::Boolean,
        quaint::ValueType::Json(_) => ArgScalarType::Json,
        quaint::ValueType::DateTime(_) | quaint::ValueType::Date(_) | quaint::ValueType::Time(_) => {
            ArgScalarType::DateTime
        }
        quaint::ValueType::Array(list) => {
            let scalar_type = list
                .as_deref()
                .unwrap_or_default()
                .first()
                .map(|val| {
                    let DynamicArgType::Single { r#type } = quaint_value_to_arg_type(val) else {
                        panic!("array element type must not be a tuple");
                    };
                    assert_eq!(r#type.arity, Arity::Scalar, "array element type must be a scalar type");
                    r#type.scalar_type
                })
                .unwrap_or(ArgScalarType::Unknown);
            return DynamicArgType::Single {
                r#type: ArgType::new(Arity::List, scalar_type, native_type),
            };
        }
        quaint::ValueType::EnumArray(_, _) => {
            return DynamicArgType::Single {
                r#type: ArgType::new(Arity::List, ArgScalarType::Enum, native_type),
            };
        }
        quaint::ValueType::Opaque(opaque) => return opaque_type_to_arg_type(opaque.typ(), native_type),
    };
    DynamicArgType::Single {
        r#type: ArgType::new(Arity::Scalar, scalar_type, native_type),
    }
}

fn opaque_type_to_arg_type(opaque: &OpaqueType, native_type: Option<String>) -> DynamicArgType {
    let scalar_type = match opaque {
        OpaqueType::Unknown => ArgScalarType::Unknown,
        OpaqueType::Int32 => ArgScalarType::Int,
        OpaqueType::Int64 => ArgScalarType::BigInt,
        OpaqueType::Float | OpaqueType::Double => ArgScalarType::Float,
        OpaqueType::Numeric => ArgScalarType::Decimal,
        OpaqueType::Enum => ArgScalarType::Enum,
        OpaqueType::Text | OpaqueType::Xml | OpaqueType::Char => ArgScalarType::String,
        OpaqueType::Uuid => ArgScalarType::Uuid,
        OpaqueType::Bytes => ArgScalarType::Bytes,
        OpaqueType::Boolean => ArgScalarType::Boolean,
        OpaqueType::Json | OpaqueType::Object => ArgScalarType::Json,
        OpaqueType::DateTime | OpaqueType::Date | OpaqueType::Time => ArgScalarType::DateTime,
        OpaqueType::Array(element_type) => {
            let DynamicArgType::Single { r#type } = opaque_type_to_arg_type(element_type, native_type) else {
                panic!("array element type must not be a tuple");
            };
            assert_eq!(r#type.arity, Arity::Scalar, "array element type must be a scalar type");
            return DynamicArgType::Single {
                r#type: ArgType::new(Arity::List, r#type.scalar_type, r#type.db_type),
            };
        }
        OpaqueType::Tuple(elems) => {
            return DynamicArgType::Tuple {
                elements: elems
                    .iter()
                    .map(|(typ, nt)| {
                        let DynamicArgType::Single { r#type } =
                            opaque_type_to_arg_type(typ, nt.as_deref().map(ToOwned::to_owned))
                        else {
                            panic!("tuple element type must not be a tuple");
                        };
                        r#type
                    })
                    .collect(),
            };
        }
    };
    DynamicArgType::Single {
        r#type: ArgType::new(Arity::Scalar, scalar_type, native_type),
    }
}

pub fn type_identifier_to_opaque_type(identifier: &TypeIdentifier) -> OpaqueType {
    match identifier {
        TypeIdentifier::String => OpaqueType::Text,
        TypeIdentifier::Int => OpaqueType::Int32,
        TypeIdentifier::BigInt => OpaqueType::Int64,
        TypeIdentifier::Float => OpaqueType::Numeric,
        TypeIdentifier::Decimal => OpaqueType::Numeric,
        TypeIdentifier::Boolean => OpaqueType::Boolean,
        TypeIdentifier::Enum(_) => OpaqueType::Enum,
        TypeIdentifier::UUID => OpaqueType::Uuid,
        TypeIdentifier::Json => OpaqueType::Json,
        TypeIdentifier::DateTime => OpaqueType::DateTime,
        TypeIdentifier::Bytes => OpaqueType::Bytes,
        TypeIdentifier::Unsupported => OpaqueType::Unknown,
    }
}

pub fn type_information_to_opaque_type(typ: &FieldTypeInformation) -> OpaqueType {
    match typ.arity {
        FieldArity::Required | FieldArity::Optional => type_identifier_to_opaque_type(&typ.typ.id),
        FieldArity::List => OpaqueType::Array(type_identifier_to_opaque_type(&typ.typ.id).into()),
    }
}
