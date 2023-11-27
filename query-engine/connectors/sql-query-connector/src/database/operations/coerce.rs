use itertools::{Either, Itertools};
use query_structure::*;

use crate::query_arguments_ext::QueryArgumentsExt;

// TODO: find better name
pub(crate) fn coerce_record_with_join(record: &mut Record, rq_indexes: Vec<(usize, &RelationSelection)>) {
    for (val_idx, rs) in rq_indexes {
        let val = record.values.get_mut(val_idx).unwrap();
        // TODO(perf): Find ways to avoid serializing and deserializing multiple times.
        let json_val: serde_json::Value = serde_json::from_str(val.as_json().unwrap()).unwrap();

        *val = coerce_json_relation_to_pv(json_val, rs);
    }
}

// TODO: find better name
pub(crate) fn coerce_json_relation_to_pv(value: serde_json::Value, rs: &RelationSelection) -> PrismaValue {
    let relations = rs.relations().collect_vec();

    match value {
        // one-to-many
        serde_json::Value::Array(values) if rs.field.is_list() => {
            let iter = values.into_iter().map(|value| coerce_json_relation_to_pv(value, rs));

            // Reverses order when using negative take.
            let iter = match rs.args.needs_reversed_order() {
                true => Either::Left(iter.rev()),
                false => Either::Right(iter),
            };

            PrismaValue::List(iter.collect())
        }
        // to-one
        serde_json::Value::Array(values) => {
            let coerced = values
                .into_iter()
                .next()
                .map(|value| coerce_json_relation_to_pv(value, rs));

            // TODO(HACK): We probably want to update the sql builder instead to not aggregate to-one relations as array
            // If the arary is empty, it means there's no relations, so we coerce it to
            if let Some(val) = coerced {
                val
            } else {
                PrismaValue::Null
            }
        }
        serde_json::Value::Object(obj) => {
            let mut map: Vec<(String, PrismaValue)> = Vec::with_capacity(obj.len());
            let related_model = rs.field.related_model();

            for (key, value) in obj {
                match related_model.fields().all().find(|f| f.db_name() == key).unwrap() {
                    Field::Scalar(sf) => {
                        map.push((key, coerce_json_scalar_to_pv(value, &sf)));
                    }
                    Field::Relation(rf) => {
                        // TODO: optimize this
                        if let Some(nested_selection) = relations.iter().find(|rs| rs.field == rf) {
                            map.push((key, coerce_json_relation_to_pv(value, nested_selection)));
                        }
                    }
                    _ => (),
                }
            }

            PrismaValue::Object(map)
        }
        _ => unreachable!(),
    }
}

pub(crate) fn coerce_json_scalar_to_pv(value: serde_json::Value, sf: &ScalarField) -> PrismaValue {
    match value {
        serde_json::Value::Null => PrismaValue::Null,
        serde_json::Value::Bool(b) => PrismaValue::Boolean(b),
        serde_json::Value::Number(n) => match sf.type_identifier() {
            TypeIdentifier::Int => PrismaValue::Int(n.as_i64().unwrap()),
            TypeIdentifier::BigInt => PrismaValue::BigInt(n.as_i64().unwrap()),
            TypeIdentifier::Float => todo!(),
            TypeIdentifier::Decimal => todo!(),
            _ => unreachable!(),
        },
        serde_json::Value::String(s) => match sf.type_identifier() {
            TypeIdentifier::String => PrismaValue::String(s),
            TypeIdentifier::Enum(_) => PrismaValue::Enum(s),
            TypeIdentifier::DateTime => PrismaValue::DateTime(parse_datetime(&s).unwrap()),
            TypeIdentifier::UUID => PrismaValue::Uuid(uuid::Uuid::parse_str(&s).unwrap()),
            TypeIdentifier::Bytes => PrismaValue::Bytes(decode_bytes(&s).unwrap()),
            _ => unreachable!(),
        },
        serde_json::Value::Array(_) => todo!(),
        serde_json::Value::Object(_) => todo!(),
    }
}
