use super::*;
use crate::{
    schema::{IntoArc, ObjectTypeStrongRef, OutputType, OutputTypeRef, ScalarType},
    CoreError, EnumType, OutputFieldRef, QueryResult, RecordAggregation, RecordSelection,
};
use connector::AggregationResult;
use indexmap::IndexMap;
use prisma_models::{InternalEnum, PrismaValue, RecordProjection};
use rust_decimal::prelude::ToPrimitive;
use std::{borrow::Borrow, collections::HashMap};

/// A grouping of items to their parent record.
/// The item implicitly holds the information of the type of item contained.
/// E.g., if the output type of a field designates a single object, the item will be
/// Item::Map(map), if it's a list, Item::List(list), etc. (hence "checked")
type CheckedItemsWithParents = IndexMap<Option<RecordProjection>, Item>;

/// A grouping of items to their parent record.
/// As opposed to the checked mapping, this map isn't holding final information about
/// the contained items, i.e. the Items are all unchecked.
type UncheckedItemsWithParents = IndexMap<Option<RecordProjection>, Vec<Item>>;

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
    field: &OutputFieldRef,
    is_list: bool,
) -> crate::Result<CheckedItemsWithParents> {
    match result {
        QueryResult::RecordSelection(rs) => serialize_record_selection(rs, field, &field.field_type, is_list),
        QueryResult::RecordAggregation(ra) => serialize_aggregation(ra),

        QueryResult::Count(c) => {
            // Todo needs a real implementation or needs to move to RecordAggregation
            let mut map: Map = IndexMap::with_capacity(1);
            let mut result = CheckedItemsWithParents::new();

            map.insert("count".into(), Item::Value(PrismaValue::Int(c as i64)));
            result.insert(None, Item::Map(map));

            Ok(result)
        }

        QueryResult::Json(_) => unimplemented!(),
        QueryResult::Id(_) => unimplemented!(),
        QueryResult::Unit => unimplemented!(),
    }
}

fn serialize_aggregation(record_aggregation: RecordAggregation) -> crate::Result<CheckedItemsWithParents> {
    let ordering = record_aggregation.selection_order;
    let results = record_aggregation.results;

    let mut flattened = HashMap::with_capacity(ordering.len());

    for result in results {
        match result {
            AggregationResult::Count(count) => {
                flattened.insert("count".to_owned(), Item::Value(count));
            }

            AggregationResult::Average(field, value) => {
                flattened.insert(format!("avg_{}", &field.name), Item::Value(value));
            }

            AggregationResult::Sum(field, value) => {
                flattened.insert(format!("sum_{}", &field.name), Item::Value(value));
            }

            AggregationResult::Min(field, value) => {
                flattened.insert(format!("min_{}", &field.name), Item::Value(value));
            }

            AggregationResult::Max(field, value) => {
                flattened.insert(format!("max_{}", &field.name), Item::Value(value));
            }
        }
    }

    // Reorder fields based on the original query selection.
    let mut inner_map: Map = IndexMap::with_capacity(ordering.len());
    for (query, field_order) in ordering {
        if let Some(order) = field_order {
            let mut nested_map = Map::new();

            for field in order {
                let item = flattened.remove(&format!("{}_{}", query, field)).unwrap();
                nested_map.insert(field, item);
            }

            inner_map.insert(query, Item::Map(nested_map));
        } else {
            let item = flattened.remove(&query).unwrap();
            inner_map.insert(query, item);
        }
    }

    let mut envelope = CheckedItemsWithParents::new();
    envelope.insert(None, Item::Map(inner_map));

    Ok(envelope)
}

fn serialize_record_selection(
    record_selection: RecordSelection,
    field: &OutputFieldRef,
    typ: &OutputTypeRef, // We additionally pass the type to allow recursing into nested type definitions of a field.
    is_list: bool,
) -> crate::Result<CheckedItemsWithParents> {
    let name = record_selection.name.clone();

    match typ.borrow() {
        OutputType::List(inner) => serialize_record_selection(record_selection, field, inner, true),
        OutputType::Object(obj) => {
            let result = serialize_objects(record_selection, obj.into_arc())?;
            let is_optional = !field.is_required;

            // Items will be ref'ed on the top level to allow cheap clones in nested scenarios.
            match (is_list, is_optional) {
                // List(Opt(_)) | List(_)
                (true, opt) => {
                    result
                        .into_iter()
                        .map(|(parent, items)| {
                            if !opt {
                                // Check that all items are non-null
                                if items.iter().any(|item| matches!(item, Item::Value(PrismaValue::Null))) {
                                    return Err(CoreError::SerializationError(format!(
                                        "Required field '{}' returned a null record",
                                        name
                                    )));
                                }
                            }

                            Ok((parent, Item::Ref(ItemRef::new(Item::list(items)))))
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
                                items.reverse();
                                let first = items.pop().unwrap();

                                // Simple return the first record in the list.
                                Ok((parent, Item::Ref(ItemRef::new(first))))
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
fn serialize_objects(
    mut result: RecordSelection,
    typ: ObjectTypeStrongRef,
) -> crate::Result<UncheckedItemsWithParents> {
    // The way our query execution works, we only need to look at nested + lists if we hit an object.
    // Move nested out of result for separate processing.
    let nested = std::mem::replace(&mut result.nested, Vec::new());

    // { <nested field name> -> { parent ID -> items } }
    let mut nested_mapping: HashMap<String, CheckedItemsWithParents> = process_nested_results(nested, &typ)?;

    // We need the Arcs to solve the issue where we have multiple parents claiming the same data (we want to move the data out of the nested structure
    // to prevent expensive copying during serialization).

    // Finally, serialize the objects based on the selected fields.
    let mut object_mapping = UncheckedItemsWithParents::with_capacity(result.scalars.records.len());
    let scalar_db_field_names = result.scalars.field_names;

    let model = result.model_id.model();
    let field_names: Vec<_> = scalar_db_field_names
        .iter()
        .filter_map(|f| model.map_scalar_db_field_name(f).map(|x| x.name.clone()))
        .collect();

    // Write all fields, nested and list fields unordered into a map, afterwards order all into the final order.
    // If nothing is written to the object, write null instead.
    for record in result.scalars.records.into_iter() {
        let record_id = Some(record.projection(&scalar_db_field_names, &result.model_id)?);

        if !object_mapping.contains_key(&record.parent_id) {
            object_mapping.insert(record.parent_id.clone(), Vec::new());
        }

        // Write scalars, but skip objects and lists, which while they are in the selection, are handled separately.
        let values = record.values;
        let mut object = HashMap::with_capacity(values.len());

        for (val, scalar_field_name) in values.into_iter().zip(field_names.iter()) {
            let field = typ.find_field(scalar_field_name).unwrap();

            if !field.field_type.is_object() {
                object.insert(scalar_field_name.to_owned(), serialize_scalar(&field, val)?);
            }
        }

        // Write nested results
        write_nested_items(&record_id, &mut nested_mapping, &mut object, &typ);

        let map = result
            .fields
            .iter()
            .fold(Map::with_capacity(result.fields.len()), |mut acc, field_name| {
                acc.insert(field_name.to_owned(), object.remove(field_name).unwrap());
                acc
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
    record_id: &Option<RecordProjection>,
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
                    OutputType::List(_) => Item::list(Vec::new()),
                    _ if !field.is_required => Item::Value(PrismaValue::Null),
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
) -> crate::Result<HashMap<String, CheckedItemsWithParents>> {
    // For each nested selected field we need to map the parents to their items.
    let mut nested_mapping = HashMap::with_capacity(nested.len());

    // Parse and validate all nested objects with their respective output type.
    // Unwraps are safe due to query validation.
    for nested_result in nested {
        // todo Workaround, tb changed with flat reads.
        if let QueryResult::RecordSelection(ref rs) = nested_result {
            let name = rs.name.clone();
            let field = enclosing_type.find_field(&name).unwrap();
            let result = serialize_internal(nested_result, &field, false)?;

            nested_mapping.insert(name, result);
        }
    }

    Ok(nested_mapping)
}

fn serialize_scalar(field: &OutputFieldRef, value: PrismaValue) -> crate::Result<Item> {
    match (&value, field.field_type.as_ref()) {
        (PrismaValue::Null, _) if !field.is_required => Ok(Item::Value(PrismaValue::Null)),
        (_, OutputType::Enum(et)) => match et.borrow() {
            EnumType::Internal(ref i) => convert_enum(value, i),
            _ => unreachable!(),
        },
        (PrismaValue::List(_), OutputType::List(arc_type)) => match arc_type.as_ref() {
            OutputType::Scalar(subtype) => {
                let items = unwrap_prisma_value(value)
                    .into_iter()
                    .map(|v| convert_prisma_value(v, subtype))
                    .map(|pv| pv.map(Item::Value))
                    .collect::<Result<Vec<Item>, CoreError>>()?;
                Ok(Item::list(items))
            }
            OutputType::Enum(et) => {
                let items = unwrap_prisma_value(value)
                    .into_iter()
                    .map(|v| match et.borrow() {
                        EnumType::Internal(ref i) => convert_enum(v, i),
                        _ => unreachable!(),
                    })
                    .collect::<Result<Vec<Item>, CoreError>>()?;
                Ok(Item::list(items))
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

        (ScalarType::Json, PrismaValue::String(s)) => PrismaValue::Json(s),
        (ScalarType::Json, PrismaValue::Json(s)) => PrismaValue::Json(s),

        (ScalarType::Int, PrismaValue::Float(f)) => PrismaValue::Int(f.to_i64().unwrap()),
        (ScalarType::Int, PrismaValue::Int(i)) => PrismaValue::Int(i),

        (ScalarType::Float, PrismaValue::Float(f)) => PrismaValue::Float(f),
        (ScalarType::Float, PrismaValue::Int(i)) => {
            PrismaValue::Int(i.to_i64().expect("Unable to convert Decimal to i64."))
        }

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

fn convert_enum(value: PrismaValue, i: &InternalEnum) -> Result<Item, CoreError> {
    match value {
        PrismaValue::String(s) | PrismaValue::Enum(s) => match i.map_output_value(&s) {
            Some(inum) => Ok(Item::Value(inum)),
            None => Err(CoreError::SerializationError(format!(
                "Value '{}' not found in enum '{:?}'",
                s, i
            ))),
        },

        val => Err(CoreError::SerializationError(format!(
            "Attempted to serialize non-enum-compatible value '{}' with enum '{:?}'",
            val, i
        ))),
    }
}

fn unwrap_prisma_value(pv: PrismaValue) -> Vec<PrismaValue> {
    match pv {
        PrismaValue::List(l) => l,
        _ => panic!("We want lists!"),
    }
}
