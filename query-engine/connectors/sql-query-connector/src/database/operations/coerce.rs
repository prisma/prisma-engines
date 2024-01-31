use bigdecimal::{BigDecimal, FromPrimitive, ParseBigDecimalError};
use itertools::{Either, Itertools};
use query_structure::*;
use std::{io, str::FromStr};

use crate::{query_arguments_ext::QueryArgumentsExt, SqlError};

/// Coerces relations resolved as JSON to PrismaValues.
/// Note: Some in-memory processing is baked into this function too for performance reasons.
pub(crate) fn coerce_record_with_json_relation(
    record: &mut Record,
    rs_indexes: &[(usize, &RelationSelection)],
    vs_indexes: &[(usize, &str)],
) -> crate::Result<()> {
    for &(val_idx, rs) in rs_indexes {
        let val = record.values.get_mut(val_idx).unwrap();
        // TODO(perf): Find ways to avoid serializing and deserializing multiple times.
        let json_val: serde_json::Value = serde_json::from_str(val.as_json().unwrap()).unwrap();

        *val = coerce_json_relation_to_pv(json_val, rs)?;
    }

    // TODO: merge rs_indexes and vs_indexes and have a single loop
    for &(val_idx, obj_name) in vs_indexes {
        let val = record.values.get_mut(val_idx).unwrap();
        // TODO(perf): Find ways to avoid serializing and deserializing multiple times.
        let json_val: serde_json::Value = serde_json::from_str(val.as_json().unwrap()).unwrap();

        *val = coerce_json_virtual_field_to_pv(obj_name, json_val)?;
    }

    Ok(())
}

fn coerce_json_relation_to_pv(value: serde_json::Value, rs: &RelationSelection) -> crate::Result<PrismaValue> {
    let relations = rs.relations().collect_vec();

    match value {
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

            // Reverses order when using negative take.
            let iter = match rs.args.needs_reversed_order() {
                true => Either::Left(iter.rev()),
                false => Either::Right(iter),
            };

            Ok(PrismaValue::List(iter.collect::<crate::Result<Vec<_>>>()?))
        }
        // to-one
        serde_json::Value::Array(values) => {
            let coerced = values
                .into_iter()
                .next()
                .map(|value| coerce_json_relation_to_pv(value, rs));

            // TODO(HACK): We probably want to update the sql builder instead to not aggregate to-one relations as array
            // If the array is empty, it means there's no relations, so we coerce it to
            if let Some(val) = coerced {
                val
            } else {
                Ok(PrismaValue::Null)
            }
        }
        serde_json::Value::Object(obj) => {
            let mut map: Vec<(String, PrismaValue)> = Vec::with_capacity(obj.len());
            let related_model = rs.field.related_model();

            for (key, value) in obj {
                match related_model.fields().all().find(|f| f.db_name() == key) {
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
                let bd = n
                    .as_f64()
                    .and_then(BigDecimal::from_f64)
                    .map(|bd| bd.normalized())
                    .ok_or_else(|| {
                        build_conversion_error(sf, &format!("Number({n})"), &format!("{:?}", sf.type_identifier()))
                    })?;

                Ok(PrismaValue::Float(bd))
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
            TypeIdentifier::Decimal => {
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
                // We skip the first two characters because there's the \x prefix.
                let bytes = hex::decode(&s[2..]).map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                Ok(PrismaValue::Bytes(bytes))
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
