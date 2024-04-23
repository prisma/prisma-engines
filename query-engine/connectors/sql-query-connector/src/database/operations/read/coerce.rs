use bigdecimal::{BigDecimal, ParseBigDecimalError};
use itertools::Itertools;
use query_structure::*;
use std::{borrow::Cow, io, str::FromStr};

use crate::SqlError;

use super::process::InMemoryProcessorForJoins;

pub(crate) enum IndexedSelection<'a> {
    Relation(&'a RelationSelection),
    Virtual(&'a str),
}

/// Coerces relations resolved as JSON to PrismaValues.
/// Note: Some in-memory processing is baked into this function too for performance reasons.
pub(crate) fn coerce_record_with_json_relation(
    record: &mut Record,
    indexes: &[(usize, IndexedSelection<'_>)],
) -> crate::Result<()> {
    for (val_idx, kind) in indexes {
        let val = record.values.get_mut(*val_idx).unwrap();

        match kind {
            IndexedSelection::Relation(rs) => {
                match val {
                    PrismaValue::Null if rs.field.is_list() => {
                        *val = PrismaValue::List(vec![]);
                    }
                    PrismaValue::Null if rs.field.is_optional() => {
                        continue;
                    }
                    val => {
                        // TODO(perf): Find ways to avoid serializing and deserializing multiple times.
                        let json_val: serde_json::Value = serde_json::from_str(val.as_json().unwrap()).unwrap();

                        *val = coerce_json_relation_to_pv(json_val, rs)?;
                    }
                }
            }
            IndexedSelection::Virtual(name) => {
                let json_val: serde_json::Value = serde_json::from_str(val.as_json().unwrap()).unwrap();

                *val = coerce_json_virtual_field_to_pv(name, json_val)?
            }
        };
    }

    Ok(())
}

fn coerce_json_relation_to_pv(value: serde_json::Value, rs: &RelationSelection) -> crate::Result<PrismaValue> {
    let relations = rs.relations().collect_vec();

    match value {
        // Some versions of MySQL return null when offsetting by more than the number of rows available.
        serde_json::Value::Null if rs.field.is_list() => Ok(PrismaValue::List(vec![])),
        serde_json::Value::Null if rs.field.is_optional() => Ok(PrismaValue::Null),
        // one-to-many
        serde_json::Value::Array(values) if rs.field.is_list() => {
            let iter = values.into_iter().filter_map(|value| {
                // FIXME: In the case of m2m relations, the aggregation produces null values if the B side of the m2m table points to a record that doesn't exist.
                // FIXME: This only seems to happen because of a bug with `relationMode=prisma`` which doesn't cleanup the relation table properly when deleting records that belongs to a m2m relation.
                // FIXME: This hack filters down the null values from the array, but we should fix the root cause instead, if possible.
                // FIXME: In theory, the aggregated array should only contain objects, which are the joined rows.
                // FIXME: See m2m.rs::repro_16390 for a reproduction.
                if value.is_null() && rs.field.relation().is_many_to_many() {
                    None
                } else {
                    Some(coerce_json_relation_to_pv(value, rs))
                }
            });

            let iter = InMemoryProcessorForJoins::new(&rs.args, iter).process(|maybe_value| {
                maybe_value.as_ref().ok().map(|value| {
                    let object = value
                        .clone()
                        .into_object()
                        .expect("Expected coerced_json_relation_to_pv to return list of objects");

                    let (field_names, values) = object.into_iter().unzip();

                    (Cow::Owned(Record::new(values)), Cow::Owned(field_names))
                })
            });

            Ok(PrismaValue::List(iter.collect::<crate::Result<Vec<_>>>()?))
        }
        serde_json::Value::Object(obj) => {
            let mut map: Vec<(String, PrismaValue)> = Vec::with_capacity(obj.len());
            let related_model = rs.field.related_model();

            for (key, value) in obj {
                match related_model.fields().all().find(|f| f.name() == key) {
                    Some(Field::Scalar(sf)) => {
                        map.push((key, coerce_json_scalar_to_pv(value, &sf)?));
                    }
                    Some(Field::Relation(rf)) => {
                        // TODO: optimize this
                        if let Some(nested_selection) = relations.iter().find(|rs| rs.field == rf) {
                            map.push((key, coerce_json_relation_to_pv(value, nested_selection)?));
                        }
                    }
                    None => {
                        let coerced_value = coerce_json_virtual_field_to_pv(&key, value)?;
                        map.push((key, coerced_value));
                    }
                    _ => (),
                }
            }

            Ok(PrismaValue::Object(map))
        }
        x => unreachable!("Unexpected value when deserializing JSON relation data: {x:?}"),
    }
}

pub(crate) fn coerce_json_scalar_to_pv(value: serde_json::Value, sf: &ScalarField) -> crate::Result<PrismaValue> {
    if sf.type_identifier().is_json() {
        return Ok(PrismaValue::Json(serde_json::to_string(&value)?));
    }

    match value {
        serde_json::Value::Null => {
            if sf.is_list() {
                Ok(PrismaValue::List(vec![]))
            } else {
                Ok(PrismaValue::Null)
            }
        }
        serde_json::Value::Bool(b) => Ok(PrismaValue::Boolean(b)),
        serde_json::Value::Number(n) => match sf.type_identifier() {
            TypeIdentifier::Int => Ok(PrismaValue::Int(n.as_i64().ok_or_else(|| {
                build_conversion_error(sf, &format!("Number({n})"), &format!("{:?}", sf.type_identifier()))
            })?)),
            TypeIdentifier::BigInt => Ok(PrismaValue::BigInt(n.as_i64().ok_or_else(|| {
                build_conversion_error(sf, &format!("Number({n})"), &format!("{:?}", sf.type_identifier()))
            })?)),
            TypeIdentifier::Float | TypeIdentifier::Decimal => {
                let bd = parse_decimal(&n.to_string()).map_err(|_| {
                    build_conversion_error(sf, &format!("Number({n})"), &format!("{:?}", sf.type_identifier()))
                })?;

                Ok(PrismaValue::Float(bd))
            }
            TypeIdentifier::Boolean => {
                let err =
                    || build_conversion_error(sf, &format!("Number({n})"), &format!("{:?}", sf.type_identifier()));
                let i = n.as_i64().ok_or_else(err)?;

                match i {
                    0 => Ok(PrismaValue::Boolean(false)),
                    1 => Ok(PrismaValue::Boolean(true)),
                    _ => Err(err()),
                }
            }
            _ => Err(build_conversion_error(
                sf,
                &format!("Number({n})"),
                &format!("{:?}", sf.type_identifier()),
            )),
        },
        serde_json::Value::String(s) => match sf.type_identifier() {
            TypeIdentifier::String => Ok(PrismaValue::String(s)),
            TypeIdentifier::Enum(_) => Ok(PrismaValue::Enum(s)),
            TypeIdentifier::DateTime => {
                let res = sf.parse_json_datetime(&s).map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                Ok(PrismaValue::DateTime(res))
            }
            TypeIdentifier::Decimal | TypeIdentifier::Float => {
                let res = parse_decimal(&s).map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                Ok(PrismaValue::Float(res))
            }
            TypeIdentifier::UUID => Ok(PrismaValue::Uuid(uuid::Uuid::parse_str(&s).map_err(|err| {
                build_conversion_error_with_reason(
                    sf,
                    &format!("String({s})"),
                    &format!("{:?}", sf.type_identifier()),
                    &err.to_string(),
                )
            })?)),
            TypeIdentifier::Bytes => {
                let bytes = sf.parse_json_bytes(&s).map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                Ok(PrismaValue::Bytes(bytes))
            }
            // Oid is returned as string
            TypeIdentifier::Int => {
                let res = s.parse::<i64>().map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                Ok(PrismaValue::Int(res))
            }
            _ => Err(build_conversion_error(
                sf,
                &format!("String({s})"),
                &format!("{:?}", sf.type_identifier()),
            )),
        },
        serde_json::Value::Array(values) => Ok(PrismaValue::List(
            values
                .into_iter()
                .map(|v| coerce_json_scalar_to_pv(v, sf))
                .collect::<crate::Result<Vec<_>>>()?,
        )),
        serde_json::Value::Object(_) => unreachable!("Objects should be caught by the json catch-all above."),
    }
}

fn coerce_json_virtual_field_to_pv(key: &str, value: serde_json::Value) -> crate::Result<PrismaValue> {
    match value {
        serde_json::Value::Object(obj) => {
            let values: crate::Result<Vec<_>> = obj
                .into_iter()
                .map(|(key, value)| coerce_json_virtual_field_to_pv(&key, value).map(|value| (key, value)))
                .collect();
            Ok(PrismaValue::Object(values?))
        }

        serde_json::Value::Number(num) => num
            .as_i64()
            .ok_or_else(|| {
                build_generic_conversion_error(format!(
                    "Unexpected numeric value {num} for virtual field '{key}': only integers are supported"
                ))
            })
            .map(PrismaValue::Int),

        _ => Err(build_generic_conversion_error(format!(
            "Field '{key}' is not a model field and doesn't have a supported type for a virtual field"
        ))),
    }
}

fn build_conversion_error(sf: &ScalarField, from: &str, to: &str) -> SqlError {
    let container_name = sf.container().name();
    let field_name = sf.name();

    build_generic_conversion_error(format!(
        "Unexpected conversion failure for field {container_name}.{field_name} from {from} to {to}."
    ))
}

fn build_conversion_error_with_reason(sf: &ScalarField, from: &str, to: &str, reason: &str) -> SqlError {
    let container_name = sf.container().name();
    let field_name = sf.name();

    build_generic_conversion_error(format!(
        "Unexpected conversion failure for field {container_name}.{field_name} from {from} to {to}. Reason: {reason}"
    ))
}

fn build_generic_conversion_error(message: String) -> SqlError {
    let error = io::Error::new(io::ErrorKind::InvalidData, message);
    SqlError::ConversionError(error.into())
}

fn parse_decimal(str: &str) -> std::result::Result<BigDecimal, ParseBigDecimalError> {
    BigDecimal::from_str(str).map(|bd| bd.normalized())
}
