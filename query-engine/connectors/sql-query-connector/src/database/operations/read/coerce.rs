use bigdecimal::{BigDecimal, FromPrimitive, ParseBigDecimalError, ToPrimitive};
use query_structure::*;
use serde_json::Number;
use std::{io, str::FromStr};

use crate::{query_arguments_ext::QueryArgumentsExt, SqlError};

pub(crate) fn coerce_json_relation_to_pv(
    mut value: serde_json::Value,
    rs: &RelationSelection,
) -> crate::Result<serde_json::Value> {
    internal_coerce_json_relation_to_pv(&mut value, rs)?;

    Ok(value)
}

fn internal_coerce_json_relation_to_pv(value: &mut serde_json::Value, rs: &RelationSelection) -> crate::Result<()> {
    match value {
        serde_json::Value::Null if rs.field.is_list() => {
            *value = serde_json::Value::Array(vec![]);
        }
        serde_json::Value::Array(values) if rs.field.is_list() => {
            let mut ids_to_remove = vec![];

            for (i, val) in values.iter_mut().enumerate() {
                if val.is_null() && rs.field.relation().is_many_to_many() {
                    ids_to_remove.push(i);
                } else {
                    internal_coerce_json_relation_to_pv(val, rs)?;
                }
            }

            for id in ids_to_remove {
                values.remove(id);
            }

            if rs.args.needs_reversed_order() {
                values.reverse();
            }
        }
        serde_json::Value::Object(obj) => {
            let mut new_obj = serde_json::Map::with_capacity(obj.len());

            for field in rs.grouped_fields_to_serialize() {
                match field {
                    GroupedSelectedField::Scalar(sf) => {
                        let (field_name, mut obj_val) = obj.remove_entry(sf.name()).unwrap();

                        coerce_json_scalar_to_pv(&mut obj_val, sf)?;

                        new_obj.insert(field_name, obj_val);
                    }
                    GroupedSelectedField::Relation(nested_rs) => {
                        let (field_name, mut obj_val) = obj.remove_entry(nested_rs.field.name()).unwrap();

                        internal_coerce_json_relation_to_pv(&mut obj_val, nested_rs)?;

                        new_obj.insert(field_name, obj_val);
                    }
                    GroupedSelectedField::Virtual(vs) => {
                        let (field_name, obj_val) = obj.remove_entry(vs.serialized_name().0).unwrap();

                        new_obj.insert(field_name, reorder_virtuals_group(obj_val, &vs));
                    }
                }
            }

            *obj = new_obj;
        }
        _ => (),
    };

    Ok(())
}

fn coerce_json_scalar_to_pv(value: &mut serde_json::Value, sf: &ScalarField) -> crate::Result<()> {
    if sf.type_identifier().is_json() {
        *value = serde_json::Value::String(value.to_string());
        return Ok(());
    }

    match value {
        serde_json::Value::Null => {
            if sf.is_list() {
                *value = serde_json::Value::Array(vec![]);
            }
        }
        serde_json::Value::Number(ref n) => match sf.type_identifier() {
            TypeIdentifier::Decimal => {
                let bd = parse_json_f64(n, sf)?;

                *value = serde_json::Value::String(bd.normalized().to_string());
            }
            TypeIdentifier::Float => {
                let bd = parse_json_f64(n, sf)?;

                *value = serde_json::Value::Number(Number::from_f64(stringify_decimal(&bd)).unwrap());
            }
            TypeIdentifier::Boolean => {
                let err =
                    || build_conversion_error(sf, &format!("Number({n})"), &format!("{:?}", sf.type_identifier()));
                let i = n.as_i64().ok_or_else(err)?;

                match i {
                    0 => *value = serde_json::Value::Bool(false),
                    1 => *value = serde_json::Value::Bool(true),
                    _ => return Err(err()),
                }
            }
            TypeIdentifier::BigInt => {
                let i = n.as_i64().ok_or_else(|| {
                    build_conversion_error(sf, &format!("Number({n})"), &format!("{:?}", sf.type_identifier()))
                })?;

                *value = serde_json::Value::String(i.to_string());
            }
            _ => (),
        },
        serde_json::Value::String(s) => match sf.type_identifier() {
            TypeIdentifier::DateTime => {
                let res = sf.parse_json_datetime(s).map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                *value = serde_json::Value::String(stringify_datetime(&res));
            }
            TypeIdentifier::Decimal => {
                let res = parse_decimal(s).map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                *value = serde_json::Value::String(res.to_string());
            }
            TypeIdentifier::Float => {
                let res = parse_decimal(s).map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                *value = serde_json::Value::Number(Number::from_f64(res.to_f64().unwrap()).unwrap());
            }
            TypeIdentifier::Bytes => {
                let bytes = sf.parse_json_bytes(s).map_err(|err| {
                    build_conversion_error_with_reason(
                        sf,
                        &format!("String({s})"),
                        &format!("{:?}", sf.type_identifier()),
                        &err.to_string(),
                    )
                })?;

                *value = serde_json::Value::String(encode_bytes(&bytes));
            }
            _ => (),
        },
        serde_json::Value::Array(values) => {
            for val in values.iter_mut() {
                coerce_json_scalar_to_pv(val, sf)?;
            }
        }
        _ => (),
    };

    Ok(())
}

pub fn reorder_virtuals_group(val: serde_json::Value, vs: &GroupedVirtualSelection) -> serde_json::Value {
    match val {
        serde_json::Value::Object(mut obj) => {
            let mut new_obj = serde_json::Map::with_capacity(obj.len());

            match vs {
                GroupedVirtualSelection::RelationCounts(rcs) => {
                    for rc in rcs {
                        let (field_name, obj_val) = obj.remove_entry(rc.field().name()).unwrap();

                        new_obj.insert(field_name, obj_val);
                    }
                }
            }

            new_obj.into()
        }
        _ => val,
    }
}

fn parse_json_f64(n: &Number, sf: &Zipper<ScalarFieldId>) -> crate::Result<BigDecimal> {
    n.as_f64()
        .and_then(BigDecimal::from_f64)
        .map(|bd| bd.normalized())
        .ok_or_else(|| build_conversion_error(sf, &format!("Number({n})"), &format!("{:?}", sf.type_identifier())))
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
