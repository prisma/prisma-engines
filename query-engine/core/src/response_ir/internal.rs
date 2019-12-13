use super::*;
use crate::{
    schema::{IntoArc, ObjectTypeStrongRef, OutputType, OutputTypeRef, ScalarType},
    CoreError, CoreResult, QueryResult, RecordSelection,
};
use indexmap::IndexMap;
use prisma_models::{EnumType, EnumValue, GraphqlId, PrismaValue};
use rust_decimal::prelude::ToPrimitive;
use std::{borrow::Borrow, collections::HashMap, convert::TryFrom};

/// A grouping of items to their parent record.
/// The item implicitly holds the information of the type of item contained.
/// E.g., if the output type of a field designates a single object, the item will be
/// Item::Map(map), if it's a list, Item::List(list), etc. (hence "checked")
type CheckedItemsWithParents = IndexMap<Option<GraphqlId>, Item>;

/// A grouping of items to their parent record.
/// As opposed to the checked mapping, this map isn't holding final information about
/// the contained items, i.e. the Items are all unchecked.
type UncheckedItemsWithParents = IndexMap<Option<GraphqlId>, Vec<Item>>;

/// The query validation makes sure that the output selection already has the correct shape.
/// This means that we can make the following assumptions:
/// - Objects don't need to check required fields.
/// - Objects don't need to check extra fields - just pick the selected ones and ignore the rest.
///
/// The output validation has to make sure that returned values:
/// - Are of the correct type.
/// - Are nullable if not present.
///
/// The is_list and is_optional flags dictate how object checks are done.
/// // todo more here
///
/// Returns a map of pairs of (parent ID, response)
pub fn serialize_internal(
    result: QueryResult,
    typ: &OutputTypeRef,
    is_list: bool,
    is_optional: bool,
) -> CoreResult<CheckedItemsWithParents> {
    match result {
        QueryResult::RecordSelection(rs) => serialize_record_selection(rs, typ, is_list, is_optional),

        QueryResult::Count(c) => {
            // Todo needs a real implementation
            let mut map: IndexMap<String, Item> = IndexMap::new();
            let mut result = CheckedItemsWithParents::new();

            map.insert("count".into(), Item::Value(PrismaValue::Int(c as i64)));
            result.insert(None, Item::Map(map));

            Ok(result)
        }

        QueryResult::Id(_) => unimplemented!(),
        QueryResult::Unit => unimplemented!(),
    }
}

fn serialize_record_selection(
    record_selection: RecordSelection,
    typ: &OutputTypeRef,
    is_list: bool,
    is_optional: bool,
) -> CoreResult<CheckedItemsWithParents> {
    let query_args = record_selection.query_arguments.clone();
    let name = record_selection.name.clone();

    match typ.borrow() {
        OutputType::List(inner) => serialize_record_selection(record_selection, inner, true, false),
        OutputType::Opt(inner) => serialize_record_selection(record_selection, inner, is_list, true),
        OutputType::Object(obj) => {
            let result = serialize_objects(record_selection, obj.into_arc())?;

            // Items will be ref'ed on the top level to allow cheap clones in nested scenarios.
            match (is_list, is_optional) {
                // List(Opt(_)) | List(_)
                (true, opt) => {
                    result
                        .into_iter()
                        .map(|(parent, mut items)| {
                            if !opt {
                                // Check that all items are non-null
                                if items.iter().any(|item| match item {
                                    Item::Value(PrismaValue::Null) => true,
                                    _ => false,
                                }) {
                                    return Err(CoreError::SerializationError(format!(
                                        "Required field '{}' returned a null record",
                                        name
                                    )));
                                }
                            }

                            // Trim excess records
                            trim_records(&mut items, &query_args);
                            Ok((parent, Item::Ref(ItemRef::new(Item::List(items)))))
                        })
                        .collect()
                }

                // Opt(_)
                (false, opt) => {
                    result
                        .into_iter()
                        .map(|(parent, mut items)| {
                            // As it's not a list, we require a single result
                            if items.len() > 1 {
                                Err(CoreError::SerializationError(format!(
                                    "Expected at most 1 item for '{}', got {}",
                                    name,
                                    items.len()
                                )))
                            } else if items.is_empty() && opt {
                                Ok((parent, Item::Ref(ItemRef::new(Item::Value(PrismaValue::Null)))))
                            } else if items.is_empty() && opt {
                                Err(CoreError::SerializationError(format!(
                                    "Required field '{}' returned a null record",
                                    name
                                )))
                            } else {
                                Ok((parent, Item::Ref(ItemRef::new(items.pop().unwrap()))))
                            }
                        })
                        .collect()
                }
            }
        }

        _ => unreachable!(), // We always serialize record selections into objects or lists on the top levels. Scalars and enums are handled separately.
    }
}

/// Serializes the given result into objects of given type.
/// Doesn't validate the shape of the result set ("unchecked" result).
/// Returns a vector of serialized objects (as Item::Map), grouped into a map by parent, if present.
fn serialize_objects(mut result: RecordSelection, typ: ObjectTypeStrongRef) -> CoreResult<UncheckedItemsWithParents> {
    // The way our query execution works, we only need to look at nested + lists if we hit an object.
    // Move nested out of result for separate processing.
    let nested = std::mem::replace(&mut result.nested, vec![]);

    // { <nested field name> -> { parent ID -> items } }
    let mut nested_mapping: HashMap<String, CheckedItemsWithParents> = process_nested_results(nested, &typ)?;

    // We need the Arcs to solve the issue where we have multiple parents claiming the same data (we want to move the data out of the nested structure
    // to prevent expensive copying during serialization).

    // Finally, serialize the objects based on the selected fields.
    let mut object_mapping = UncheckedItemsWithParents::new();
    let scalar_field_names = result.scalars.field_names;

    // Write all fields, nested and list fields unordered into a map, afterwards order all into the final order.
    // If nothing is written to the object, write null instead.

    for record in result.scalars.records {
        let record_id = Some(record.collect_id(&scalar_field_names, &result.id_field)?);

        if !object_mapping.contains_key(&record.parent_id) {
            object_mapping.insert(record.parent_id.clone(), vec![]);
        }

        let mut object: HashMap<String, Item> = HashMap::new();

        // Write scalars, but skip objects and lists, which while they are in the selection, are handled separately.
        let values = record.values;
        for (val, field_name) in values.into_iter().zip(scalar_field_names.iter()) {
            let field = typ.find_field(field_name).unwrap();
            if !field.field_type.is_object() {
                object.insert(field_name.to_owned(), serialize_scalar(val, &field.field_type)?);
            }
        }

        // Write nested results
        write_nested_items(&record_id, &mut nested_mapping, &mut object, &typ);

        // Reorder into final shape.
        let mut map = Map::new();
        result.fields.iter().for_each(|field_name| {
            map.insert(field_name.to_owned(), object.remove(field_name).unwrap());
        });

        // TODO: Find out how to easily determine when a result is null.
        // If the object is null or completely empty, coerce into null instead.
        let result = Item::Map(map);
        // let result = if result.is_null_or_empty() {
        //     Item::Value(PrismaValue::Null)
        // } else {
        //     result
        // };

        object_mapping.get_mut(&record.parent_id).unwrap().push(result);
    }

    Ok(object_mapping)
}

/// Unwraps are safe due to query validation.
fn write_nested_items(
    record_id: &Option<GraphqlId>,
    items_with_parent: &mut HashMap<String, CheckedItemsWithParents>,
    into: &mut HashMap<String, Item>,
    enclosing_type: &ObjectTypeStrongRef,
) {
    items_with_parent.iter_mut().for_each(|(field_name, inner)| {
        let val = inner.get(record_id);

        // The value must be a reference (or None - handle default), everything else is an error in the serialization logic.
        match val {
            Some(Item::Ref(ref r)) => {
                into.insert(field_name.to_owned(), Item::Ref(ItemRef::clone(r)));
            }

            None => {
                let field = enclosing_type.find_field(field_name).unwrap();
                let default = match field.field_type.borrow() {
                    OutputType::List(_) => Item::List(vec![]),
                    OutputType::Opt(inner) => {
                        if inner.is_list() {
                            Item::List(vec![])
                        } else {
                            Item::Value(PrismaValue::Null)
                        }
                    }
                    _ => panic!(
                        "Application logic invariant error: received null value for field {} which may not be null",
                        &field_name
                    ),
                };

                into.insert(field_name.to_owned(), Item::Ref(ItemRef::new(default)));
            }
            _ => panic!("Application logic invariant error: Nested items have to be wrapped as a Item::Ref."),
        };
    });
}

/// Processes nested results into a more ergonomic structure of { <nested field name> -> { parent ID -> item (list, map, ...) } }.
fn process_nested_results(
    nested: Vec<QueryResult>,
    enclosing_type: &ObjectTypeStrongRef,
) -> CoreResult<HashMap<String, CheckedItemsWithParents>> {
    // For each nested selected field we need to map the parents to their items.
    let mut nested_mapping = HashMap::new();

    // Parse and validate all nested objects with their respective output type.
    // Unwraps are safe due to query validation.
    for nested_result in nested {
        // todo Workaround, tb changed with flat reads.
        if let QueryResult::RecordSelection(ref rs) = nested_result {
            let name = rs.name.clone();
            let field = enclosing_type.find_field(&name).unwrap();
            let result = serialize_internal(nested_result, &field.field_type, false, false)?;

            nested_mapping.insert(name, result);
        }
    }

    Ok(nested_mapping)
}

fn serialize_scalar(value: PrismaValue, typ: &OutputTypeRef) -> CoreResult<Item> {
    match (&value, typ.as_ref()) {
        (PrismaValue::Null, OutputType::Opt(_)) => Ok(Item::Value(PrismaValue::Null)),
        (_, OutputType::Opt(inner)) => serialize_scalar(value, inner),
        (_, OutputType::Enum(et)) => match value {
            PrismaValue::String(s) => match et.value_for(&s) {
                Some(ev) => Ok(Item::Value(PrismaValue::Enum(ev.clone()))),
                None => Err(CoreError::SerializationError(format!(
                    "Value '{}' not found in enum '{:?}'",
                    s, et
                ))),
            },

            PrismaValue::Enum(ref ev) => convert_enum_to_item(&ev, et),

            val => Err(CoreError::SerializationError(format!(
                "Attempted to serialize non-enum-compatible value '{}' with enum '{:?}'",
                val, et
            ))),
        },
        (PrismaValue::List(_), OutputType::List(arc_type)) => match arc_type.as_ref() {
            OutputType::Scalar(subtype) => {
                let items = unwrap_prisma_value(value)
                    .into_iter()
                    .map(|v| convert_prisma_value(v, subtype))
                    .map(|pv| pv.map(|x| Item::Value(x)))
                    .collect::<Result<Vec<Item>, CoreError>>()?;
                Ok(Item::List(items))
            }
            OutputType::Enum(subtype) => {
                let items = unwrap_prisma_value(value)
                    .into_iter()
                    .map(|v| match v {
                        PrismaValue::Enum(ref ev) => convert_enum_to_item(ev, subtype),
                        val => Err(CoreError::SerializationError(format!(
                            "Attempted to serialize non-enum-compatible value '{}' with enum '{:?}'",
                            val, subtype
                        ))),
                    })
                    .collect::<Result<Vec<Item>, CoreError>>()?;
                Ok(Item::List(items))
            }
            _ => Err(CoreError::SerializationError(format!(
                "Attempted to serialize scalar list which contained non-scalar items of type '{:?}'",
                arc_type
            ))),
        },
        (_, OutputType::Scalar(st)) => Ok(Item::Value(convert_prisma_value(value, st)?)),
        (pv, ot) => Err(CoreError::SerializationError(format!(
            "Attempted to serialize scalar '{}' with non-scalar compatible type '{:?}'",
            pv, ot
        ))),
    }
}

fn convert_prisma_value(value: PrismaValue, st: &ScalarType) -> Result<PrismaValue, CoreError> {
    let item_value = match (st, value) {
        (ScalarType::String, PrismaValue::String(s)) => PrismaValue::String(s),

        (ScalarType::ID, PrismaValue::GraphqlId(id)) => PrismaValue::GraphqlId(id),
        (ScalarType::ID, val) => PrismaValue::GraphqlId(GraphqlId::try_from(val)?),

        (ScalarType::Int, PrismaValue::Float(f)) => PrismaValue::Int(f.to_i64().unwrap()),
        (ScalarType::Int, PrismaValue::Int(i)) => PrismaValue::Int(i),

        (ScalarType::Float, PrismaValue::Float(f)) => PrismaValue::Float(f),
        (ScalarType::Float, PrismaValue::Int(i)) => {
            PrismaValue::Int(i.to_i64().expect("Unable to convert Decimal to i64."))
        }

        (ScalarType::Enum(ref et), PrismaValue::Enum(ref ev)) => match et.value_for(&ev.name) {
            Some(_) => PrismaValue::Enum(ev.clone()),
            None => {
                return Err(CoreError::SerializationError(format!(
                    "Enum value '{}' not found on enum '{}'",
                    ev.as_string(),
                    et.name
                )))
            }
        },

        (ScalarType::Boolean, PrismaValue::Boolean(b)) => PrismaValue::Boolean(b),
        (ScalarType::DateTime, PrismaValue::DateTime(dt)) => PrismaValue::DateTime(dt),
        (ScalarType::UUID, PrismaValue::Uuid(u)) => PrismaValue::Uuid(u),

        (st, pv) => {
            return Err(CoreError::SerializationError(format!(
                "Attempted to serialize scalar '{}' with incompatible type '{:?}'",
                pv, st
            )))
        }
    };
    Ok(item_value)
}

//fn serialize_scalar(value: PrismaValue, typ: &OutputTypeRef) -> CoreResult<Item> {
//    match (&value, typ.as_ref()) {
//        (PrismaValue::Null, OutputType::Opt(_)) => Ok(Item::Value(PrismaValue::Null)),
//        (_, OutputType::Opt(inner)) => serialize_scalar(value, inner),
//        (_, OutputType::Enum(et)) => match value {
//            PrismaValue::String(s) => match et.value_for(&s) {
//                Some(ev) => Ok(Item::Value(PrismaValue::Enum(ev.clone()))),
//                None => Err(CoreError::SerializationError(format!(
//                    "Value '{}' not found in enum '{:?}'",
//                    s, et
//                ))),
//            },
//
//            PrismaValue::Enum(ref ev) => convert_enum_to_item(&ev, et),
//
//            val => Err(CoreError::SerializationError(format!(
//                "Attempted to serialize non-enum-compatible value '{}' with enum '{:?}'",
//                val, et
//            ))),
//        },
//<<<<<<< HEAD
//        (PrismaValue::List(_), OutputType::List(arc_type)) => match arc_type.as_ref() {
//            OutputType::Scalar(subtype) => {
//                let items = unwrap_prisma_value(value)
//                    .into_iter()
//                    .map(|v| convert_prisma_value(v, subtype))
//                    .map(|pv| pv.map(|x| Item::Value(x)))
//                    .collect::<Result<Vec<Item>, CoreError>>()?;
//                Ok(Item::List(items))
//            }
//            OutputType::Enum(subtype) => {
//                let items = unwrap_prisma_value(value)
//                    .into_iter()
//                    .map(|v| match v {
//                        PrismaValue::Enum(ref ev) => convert_enum_to_item(ev, subtype),
//                        val => Err(CoreError::SerializationError(format!(
//                            "Attempted to serialize non-enum-compatible value '{}' with enum '{:?}'",
//                            val, subtype
//                        ))),
//                    })
//                    .collect::<Result<Vec<Item>, CoreError>>()?;
//                Ok(Item::List(items))
//            }
//            _ => Err(CoreError::SerializationError(format!(
//                "Attempted to serialize scalar list which contained non-scalar items of type '{:?}'",
//                arc_type
//            ))),
//        },
//        (_, OutputType::Scalar(st)) => Ok(Item::Value(convert_prisma_value(value, st)?)),
//=======
//        (_, OutputType::Scalar(st)) => {
//            let item_value = match (st, value) {
//                (ScalarType::String, PrismaValue::String(s)) => PrismaValue::String(s),
//
//                (ScalarType::ID, PrismaValue::GraphqlId(id)) => PrismaValue::GraphqlId(id),
//                (ScalarType::ID, val) => PrismaValue::GraphqlId(GraphqlId::try_from(val)?),
//
//                (ScalarType::Int, PrismaValue::Float(f)) => {
//                    PrismaValue::Int(f.to_i64().expect("Unable to convert Decimal to i64."))
//                }
//                (ScalarType::Int, PrismaValue::Int(i)) => PrismaValue::Int(i),
//
//                (ScalarType::Float, PrismaValue::Float(f)) => PrismaValue::Float(f),
//                (ScalarType::Float, PrismaValue::Int(i)) => {
//                    PrismaValue::Float(Decimal::from_i64(i).expect("Unable to convert i64 to Decimal."))
//                }
//
//                (ScalarType::Enum(ref et), PrismaValue::Enum(ref ev)) => match et.value_for(&ev.name) {
//                    Some(_) => PrismaValue::Enum(ev.clone()),
//                    None => {
//                        return Err(CoreError::SerializationError(format!(
//                            "Enum value '{}' not found on enum '{}'",
//                            ev.as_string(),
//                            et.name
//                        )))
//                    }
//                },
//
//                (ScalarType::Boolean, PrismaValue::Boolean(b)) => PrismaValue::Boolean(b),
//                (ScalarType::DateTime, PrismaValue::DateTime(dt)) => PrismaValue::DateTime(dt),
//                (ScalarType::Json, _) => unimplemented!(),
//                (ScalarType::UUID, PrismaValue::Uuid(u)) => PrismaValue::Uuid(u),
//
//                (st, pv) => {
//                    return Err(CoreError::SerializationError(format!(
//                        "Attempted to serialize scalar '{}' with incompatible type '{:?}'",
//                        pv, st
//                    )))
//                }
//            };
//
//            Ok(Item::Value(item_value))
//        }
//>>>>>>> master
//        (pv, ot) => Err(CoreError::SerializationError(format!(
//            "Attempted to serialize scalar '{}' with non-scalar compatible type '{:?}'",
//            pv, ot
//        ))),
//    }
//}

fn convert_enum_to_item(ev: &EnumValue, et: &EnumType) -> Result<Item, CoreError> {
    match et.value_for(&ev.name) {
        Some(_) => Ok(Item::Value(PrismaValue::Enum(ev.clone()))),
        None => Err(CoreError::SerializationError(format!(
            "Enum value '{}' not found on enum '{}'",
            ev.as_string(),
            et.name
        ))),
    }
}

fn unwrap_prisma_value(pv: PrismaValue) -> Vec<PrismaValue> {
    match pv {
        PrismaValue::List(Some(l)) => l,
        _ => panic!("We want Some lists!"),
    }
}

//fn convert_prisma_value(value: PrismaValue, st: &ScalarType) -> Result<PrismaValue, CoreError> {
//    let item_value = match (st, value) {
//        (ScalarType::String, PrismaValue::String(s)) => PrismaValue::String(s),
//
//        (ScalarType::ID, PrismaValue::GraphqlId(id)) => PrismaValue::GraphqlId(id),
//        (ScalarType::ID, val) => PrismaValue::GraphqlId(GraphqlId::try_from(val)?),
//
//        (ScalarType::Int, PrismaValue::Float(f)) => PrismaValue::Int(f as i64),
//        (ScalarType::Int, PrismaValue::Int(i)) => PrismaValue::Int(i),
//
//        (ScalarType::Float, PrismaValue::Float(f)) => PrismaValue::Float(f),
//        (ScalarType::Float, PrismaValue::Int(i)) => PrismaValue::Float(i as f64),
//
//        (ScalarType::Enum(ref et), PrismaValue::Enum(ref ev)) => match et.value_for(&ev.name) {
//            Some(_) => PrismaValue::Enum(ev.clone()),
//            None => {
//                return Err(CoreError::SerializationError(format!(
//                    "Enum value '{}' not found on enum '{}'",
//                    ev.as_string(),
//                    et.name
//                )))
//            }
//        },
//
//        (ScalarType::Boolean, PrismaValue::Boolean(b)) => PrismaValue::Boolean(b),
//        (ScalarType::DateTime, PrismaValue::DateTime(dt)) => PrismaValue::DateTime(dt),
//        (ScalarType::Json, PrismaValue::Json(j)) => PrismaValue::Json(j),
//        (ScalarType::UUID, PrismaValue::Uuid(u)) => PrismaValue::Uuid(u),
//
//        (st, pv) => {
//            return Err(CoreError::SerializationError(format!(
//                "Attempted to serialize scalar '{}' with incompatible type '{:?}'",
//                pv, st
//            )))
//        }
//    };
//    Ok(item_value)
//}
