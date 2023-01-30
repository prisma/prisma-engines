use super::*;
use crate::{
    constants::custom_types, protocol::EngineProtocol, CoreError, QueryResult, RecordAggregations, RecordSelection,
};
use connector::{AggregationResult, RelAggregationResult, RelAggregationRow};
use indexmap::IndexMap;
use itertools::Itertools;
use prisma_models::{CompositeFieldRef, Field, PrismaValue, SelectionResult};
use schema::*;
use schema_builder::constants::{aggregations::*, output_fields::*};
use std::{borrow::Borrow, collections::HashMap};

/// A grouping of items to their parent record.
/// The item implicitly holds the information of the type of item contained.
/// E.g., if the output type of a field designates a single object, the item will be
/// Item::Map(map), if it's a list, Item::List(list), etc. (hence "checked")
type CheckedItemsWithParents = IndexMap<Option<SelectionResult>, Item>;

/// A grouping of items to their parent record.
/// As opposed to the checked mapping, this map isn't holding final information about
/// the contained items, i.e. the Items are all unchecked.
type UncheckedItemsWithParents = IndexMap<Option<SelectionResult>, Vec<Item>>;

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
        QueryResult::RecordSelection(rs) => serialize_record_selection(*rs, field, &field.field_type, is_list),
        QueryResult::RecordAggregations(ras) => serialize_aggregations(field, ras),
        QueryResult::Count(c) => {
            // Todo needs a real implementation or needs to move to RecordAggregation
            let mut map: Map = IndexMap::with_capacity(1);
            let mut result = CheckedItemsWithParents::new();

            map.insert(AFFECTED_COUNT.into(), Item::Value(PrismaValue::Int(c as i64)));
            result.insert(None, Item::Map(map));

            Ok(result)
        }
        QueryResult::Json(_) => unimplemented!(),
        QueryResult::Id(_) => unimplemented!(),
        QueryResult::Unit => unimplemented!(),
    }
}

fn serialize_aggregations(
    output_field: &OutputFieldRef,
    record_aggregations: RecordAggregations,
) -> crate::Result<CheckedItemsWithParents> {
    let ordering = record_aggregations.selection_order;
    let aggregate_object_type = extract_aggregate_object_type(output_field.field_type.borrow());

    let mut results = vec![];

    for row in record_aggregations.results {
        let mut flattened = HashMap::with_capacity(ordering.len());

        for result in row {
            match result {
                AggregationResult::Field(field, value) => {
                    let output_field = aggregate_object_type.find_field(field.name()).unwrap();
                    flattened.insert(field.name().to_owned(), serialize_scalar(&output_field, value)?);
                }

                AggregationResult::Count(field, count) => {
                    if let Some(f) = field {
                        flattened.insert(format!("_count_{}", f.name()), Item::Value(count));
                    } else {
                        flattened.insert("_count__all".to_owned(), Item::Value(count));
                    }
                }

                AggregationResult::Average(field, value) => {
                    let output_field =
                        find_nested_aggregate_output_field(&aggregate_object_type, UNDERSCORE_AVG, field.name());
                    flattened.insert(
                        format!("_avg_{}", field.name()),
                        serialize_scalar(&output_field, value)?,
                    );
                }

                AggregationResult::Sum(field, value) => {
                    let output_field =
                        find_nested_aggregate_output_field(&aggregate_object_type, UNDERSCORE_SUM, field.name());
                    flattened.insert(
                        format!("_sum_{}", field.name()),
                        serialize_scalar(&output_field, value)?,
                    );
                }

                AggregationResult::Min(field, value) => {
                    let output_field =
                        find_nested_aggregate_output_field(&aggregate_object_type, UNDERSCORE_MIN, field.name());
                    flattened.insert(
                        format!("_min_{}", field.name()),
                        serialize_scalar(&output_field, coerce_non_numeric(value, &output_field.field_type))?,
                    );
                }

                AggregationResult::Max(field, value) => {
                    let output_field =
                        find_nested_aggregate_output_field(&aggregate_object_type, UNDERSCORE_MAX, field.name());
                    flattened.insert(
                        format!("_max_{}", field.name()),
                        serialize_scalar(&output_field, coerce_non_numeric(value, &output_field.field_type))?,
                    );
                }
            }
        }

        // Reorder fields based on the original query selection.
        // Temporary: The original selection may be done with _ or no underscore (deprecated).
        let mut inner_map: Map = IndexMap::with_capacity(ordering.len());
        for (query, field_order) in ordering.iter() {
            if let Some(order) = field_order {
                let mut nested_map = Map::new();

                for field in order {
                    let item = flattened
                        .remove(&format!("{query}_{field}"))
                        .or_else(|| flattened.remove(&format!("_{query}_{field}")))
                        .unwrap();

                    nested_map.insert(field.clone(), item);
                }

                inner_map.insert(query.clone(), Item::Map(nested_map));
            } else {
                let item = flattened
                    .remove(&query.clone())
                    .or_else(|| flattened.remove(&format!("_{query}")))
                    .unwrap();

                inner_map.insert(query.clone(), item);
            }
        }

        results.push(Item::Map(inner_map));
    }

    let mut envelope = CheckedItemsWithParents::new();

    match output_field.field_type.borrow() {
        OutputType::List(_) => {
            envelope.insert(None, Item::List(results.into()));
        }
        OutputType::Object(_) => {
            if let Some(item) = results.pop() {
                envelope.insert(None, item);
            };
        }
        _ => unreachable!(),
    };

    Ok(envelope)
}

fn write_rel_aggregation_row(row: &RelAggregationRow, map: &mut HashMap<String, Item>) {
    for result in row.iter() {
        match result {
            RelAggregationResult::Count(rf, count) => match map.get_mut(UNDERSCORE_COUNT) {
                Some(item) => match item {
                    Item::Map(inner_map) => inner_map.insert(rf.name().to_owned(), Item::Value(count.clone())),
                    _ => unreachable!(),
                },
                None => {
                    let mut inner_map: Map = Map::new();
                    inner_map.insert(rf.name().to_owned(), Item::Value(count.clone()));
                    map.insert(UNDERSCORE_COUNT.to_owned(), Item::Map(inner_map))
                }
            },
        };
    }
}

fn extract_aggregate_object_type(output_type: &OutputType) -> ObjectTypeStrongRef {
    match output_type {
        OutputType::Object(obj) => obj.into_arc(),
        OutputType::List(inner) => extract_aggregate_object_type(inner),
        _ => unreachable!("Aggregate output must be a list or an object."),
    }
}

// Workaround until we streamline serialization.
fn find_nested_aggregate_output_field(
    object_type: &ObjectTypeStrongRef,
    nested_obj_name: &str,
    nested_field_name: &str,
) -> OutputFieldRef {
    let nested_field = object_type.find_field(nested_obj_name).unwrap();
    let nested_object_type = match nested_field.field_type.borrow() {
        OutputType::Object(obj) => obj.into_arc(),
        _ => unreachable!("{} output must be an object.", nested_obj_name),
    };

    nested_object_type.find_field(nested_field_name).unwrap()
}

fn coerce_non_numeric(value: PrismaValue, output: &OutputType) -> PrismaValue {
    match (value, output.borrow()) {
        (PrismaValue::Int(x), OutputType::Scalar(ScalarType::String)) if x == 0 => PrismaValue::Null,
        (x, _) => x,
    }
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
            let is_optional = field.is_nullable;

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
                                    return Err(CoreError::null_serialization_error(&name));
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
                                Err(CoreError::null_serialization_error(&name))
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
    let nested = std::mem::take(&mut result.nested);

    // { <nested field name> -> { parent ID -> items } }
    let mut nested_mapping: HashMap<String, CheckedItemsWithParents> = process_nested_results(nested, &typ)?;

    // We need the Arcs to solve the issue where we have multiple parents claiming the same data (we want to move the data out of the nested structure
    // to prevent expensive copying during serialization).

    // Finally, serialize the objects based on the selected fields.
    let mut object_mapping = UncheckedItemsWithParents::with_capacity(result.scalars.records.len());
    let db_field_names = result.scalars.field_names;
    let model = result.model;

    let fields: Vec<_> = db_field_names
        .iter()
        .filter_map(|f| model.fields().find_from_non_virtual_by_db_name(f).ok())
        .collect();

    // Write all fields, nested and list fields unordered into a map, afterwards order all into the final order.
    // If nothing is written to the object, write null instead.
    for (r_index, record) in result.scalars.records.into_iter().enumerate() {
        let record_id = Some(record.extract_selection_result(&db_field_names, &model.primary_identifier())?);

        if !object_mapping.contains_key(&record.parent_id) {
            object_mapping.insert(record.parent_id.clone(), Vec::new());
        }

        // Write scalars and composites, but skip objects (relations) and scalar lists, which while they are in the selection, are handled separately.
        let values = record.values;
        let mut object = HashMap::with_capacity(values.len());

        for (val, field) in values.into_iter().zip(fields.iter()) {
            let out_field = typ.find_field(field.name()).unwrap();

            match field {
                Field::Composite(cf) => {
                    object.insert(field.name().to_owned(), serialize_composite(cf, &out_field, val)?);
                }

                _ if !out_field.field_type.is_object() => {
                    object.insert(field.name().to_owned(), serialize_scalar(&out_field, val)?);
                }

                _ => (),
            }
        }

        // Write nested results
        write_nested_items(&record_id, &mut nested_mapping, &mut object, &typ)?;

        let aggr_row = result.aggregation_rows.as_ref().map(|rows| rows.get(r_index).unwrap());
        if let Some(aggr_row) = aggr_row {
            write_rel_aggregation_row(aggr_row, &mut object);
        }

        let mut aggr_fields = aggr_row
            .map(|row| {
                row.iter()
                    .map(|aggr_result| match aggr_result {
                        RelAggregationResult::Count(_, _) => UNDERSCORE_COUNT.to_owned(),
                    })
                    .unique()
                    .collect()
            })
            .unwrap_or_default();

        let mut all_fields = result.fields.clone();
        all_fields.append(&mut aggr_fields);

        let map = all_fields
            .iter()
            .fold(Map::with_capacity(all_fields.len()), |mut acc, field_name| {
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
    record_id: &Option<SelectionResult>,
    items_with_parent: &mut HashMap<String, CheckedItemsWithParents>,
    into: &mut HashMap<String, Item>,
    enclosing_type: &ObjectTypeStrongRef,
) -> crate::Result<()> {
    for (field_name, inner) in items_with_parent.iter_mut() {
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
                    _ if field.is_nullable => Item::Value(PrismaValue::Null),
                    _ => return Err(CoreError::null_serialization_error(field_name)),
                };

                into.insert(field_name.to_owned(), Item::Ref(ItemRef::new(default)));
            }
            _ => panic!("Invariant error: Nested items have to be wrapped as a Item::Ref."),
        };
    }

    Ok(())
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

// Problem: order of selections
fn serialize_composite(cf: &CompositeFieldRef, out_field: &OutputFieldRef, value: PrismaValue) -> crate::Result<Item> {
    match value {
        PrismaValue::Null if !cf.is_required() => Ok(Item::Value(PrismaValue::Null)),

        PrismaValue::List(values) if cf.is_list() => {
            let values = values
                .into_iter()
                .map(|value| serialize_composite(cf, out_field, value))
                .collect::<crate::Result<Vec<_>>>();

            Ok(Item::List(values?.into()))
        }

        PrismaValue::Object(pairs) => {
            let mut map = Map::new();
            let object_type = out_field
                .field_type
                .as_object_type()
                .expect("Composite output field is not an object.");

            let composite_type = &cf.typ;

            for (field_name, value) in pairs {
                // The field on the composite type.
                // This will cause clashes if one field has an @map("name") and the other field is named "field" directly.
                let inner_field = composite_type
                    .find_field(&field_name)
                    .or_else(|| composite_type.find_field_by_db_name(&field_name))
                    .unwrap();

                // The field on the output object type. Used for the actual serialization process.
                let inner_out_field = object_type.find_field(inner_field.name()).unwrap();

                match inner_field {
                    Field::Composite(cf) => {
                        map.insert(
                            inner_field.name().to_owned(),
                            serialize_composite(cf, &inner_out_field, value)?,
                        );
                    }

                    _ if !inner_out_field.field_type.is_object() => {
                        map.insert(
                            inner_field.name().to_owned(),
                            serialize_scalar(&inner_out_field, value)?,
                        );
                    }

                    _ => (),
                }
            }

            Ok(Item::Map(map))
        }

        val => Err(CoreError::SerializationError(format!(
            "Attempted to serialize '{}' with non-composite compatible type '{:?}' for field {}.",
            val, cf.typ.name, cf.name
        ))),
    }
}

fn serialize_scalar(field: &OutputFieldRef, value: PrismaValue) -> crate::Result<Item> {
    match (&value, field.field_type.as_ref()) {
        (PrismaValue::Null, _) if field.is_nullable => Ok(Item::Value(PrismaValue::Null)),
        (_, OutputType::Enum(et)) => match *et.into_arc() {
            EnumType::Database(ref db) => convert_enum(value, db),
            _ => unreachable!(),
        },
        (PrismaValue::List(_), OutputType::List(arc_type)) => match arc_type.as_ref() {
            OutputType::Scalar(subtype) => {
                let items = unwrap_prisma_value(value)
                    .into_iter()
                    .map(|v| convert_prisma_value(field, v, subtype))
                    .map(|pv| pv.map(Item::Value))
                    .collect::<Result<Vec<Item>, CoreError>>()?;
                Ok(Item::list(items))
            }
            OutputType::Enum(et) => {
                let items = unwrap_prisma_value(value)
                    .into_iter()
                    .map(|v| match *et.into_arc() {
                        EnumType::Database(ref dbt) => convert_enum(v, dbt),
                        _ => unreachable!(),
                    })
                    .collect::<Result<Vec<Item>, CoreError>>()?;
                Ok(Item::list(items))
            }
            _ => Err(CoreError::SerializationError(format!(
                "Attempted to serialize scalar list which contained non-scalar items of type '{:?}' for field {}.",
                arc_type, field.name
            ))),
        },
        (_, OutputType::Scalar(st)) => Ok(Item::Value(convert_prisma_value(field, value, st)?)),
        (pv, ot) => Err(CoreError::SerializationError(format!(
            "Attempted to serialize scalar '{}' with non-scalar compatible type '{:?}' for field {}.",
            pv, ot, field.name
        ))),
    }
}

fn convert_prisma_value(field: &OutputFieldRef, value: PrismaValue, st: &ScalarType) -> crate::Result<PrismaValue> {
    match crate::executor::get_engine_protocol() {
        EngineProtocol::Graphql => convert_prisma_value_graphql_protocol(field, value, st),
        EngineProtocol::Json => convert_prisma_value_json_protocol(field, value, st),
    }
}

fn convert_prisma_value_graphql_protocol(
    field: &OutputFieldRef,
    value: PrismaValue,
    st: &ScalarType,
) -> crate::Result<PrismaValue> {
    let item_value = match (st, value) {
        // Identity matchers
        (ScalarType::String, PrismaValue::String(s)) => PrismaValue::String(s),
        (ScalarType::Json, PrismaValue::Json(s)) => PrismaValue::Json(s),
        (ScalarType::Int, PrismaValue::Int(i)) => PrismaValue::Int(i),
        (ScalarType::Float, PrismaValue::Float(f)) => PrismaValue::Float(f),
        (ScalarType::BigInt, PrismaValue::BigInt(i)) => PrismaValue::BigInt(i),
        (ScalarType::Boolean, PrismaValue::Boolean(b)) => PrismaValue::Boolean(b),
        (ScalarType::DateTime, PrismaValue::DateTime(dt)) => PrismaValue::DateTime(dt),
        (ScalarType::UUID, PrismaValue::Uuid(u)) => PrismaValue::Uuid(u),
        (ScalarType::Bytes, PrismaValue::Bytes(b)) => PrismaValue::Bytes(b),
        (ScalarType::Xml, PrismaValue::Xml(b)) => PrismaValue::Xml(b),

        // The Decimal type doesn't have a corresponding PrismaValue variant. We need to serialize it
        // to String so that client can deserialize it as Decimal again.
        (ScalarType::Decimal, PrismaValue::Int(i)) => PrismaValue::String(i.to_string()),
        (ScalarType::Decimal, PrismaValue::Float(f)) => PrismaValue::String(f.to_string()),
        // TODO: Remove this, it is a hack. The Xml type no longer exists as a Prisma native type.
        // TODO: It should not exist as a ScalarType, TypeIdentifier, or PrismaValue.
        (ScalarType::String, PrismaValue::Xml(xml)) => PrismaValue::String(xml),

        (st, pv) => {
            return Err(crate::FieldConversionError::create(
                field.name.clone(),
                format!("{st:?}"),
                format!("{pv}"),
            ))
        }
    };

    Ok(item_value)
}

/// Since the JSON protocol is "schema-less" by design, clients require type information for them to
/// properly deserialize special values such as bytes, decimal, datetime, etc.
fn convert_prisma_value_json_protocol(
    field: &OutputFieldRef,
    value: PrismaValue,
    st: &ScalarType,
) -> crate::Result<PrismaValue> {
    let item_value = match (st, value) {
        // Coerced to tagged object matchers
        (ScalarType::Json, PrismaValue::Json(x)) => custom_types::make_object(custom_types::JSON, PrismaValue::Json(x)),
        (ScalarType::DateTime, PrismaValue::DateTime(x)) => {
            custom_types::make_object(custom_types::DATETIME, PrismaValue::DateTime(x))
        }
        (ScalarType::Decimal, PrismaValue::Float(x)) => {
            custom_types::make_object(custom_types::DECIMAL, PrismaValue::String(x.to_string()))
        }
        (ScalarType::Decimal, PrismaValue::Int(x)) => {
            custom_types::make_object(custom_types::DECIMAL, PrismaValue::String(x.to_string()))
        }
        (ScalarType::BigInt, PrismaValue::BigInt(x)) => {
            custom_types::make_object(custom_types::BIGINT, PrismaValue::BigInt(x))
        }
        (ScalarType::Bytes, PrismaValue::Bytes(x)) => {
            custom_types::make_object(custom_types::BYTES, PrismaValue::Bytes(x))
        }

        // Identity matchers
        (ScalarType::String, PrismaValue::String(x)) => PrismaValue::String(x),
        (ScalarType::UUID, PrismaValue::Uuid(x)) => PrismaValue::Uuid(x),
        (ScalarType::Boolean, PrismaValue::Boolean(x)) => PrismaValue::Boolean(x),
        (ScalarType::Int, PrismaValue::Int(x)) => PrismaValue::Int(x),
        (ScalarType::Float, PrismaValue::Float(x)) => PrismaValue::Float(x),

        // TODO: Xml is no longer a native Prisma type. It should not exist as special PrismaValue.
        (ScalarType::String | ScalarType::Xml, PrismaValue::Xml(xml) | PrismaValue::String(xml)) => {
            PrismaValue::String(xml)
        }

        (st, pv) => {
            return Err(crate::FieldConversionError::create(
                field.name.clone(),
                format!("{st:?}"),
                format!("{pv}"),
            ))
        }
    };

    Ok(item_value)
}

fn convert_enum(value: PrismaValue, dbt: &DatabaseEnumType) -> crate::Result<Item> {
    match value {
        PrismaValue::String(s) | PrismaValue::Enum(s) => match dbt.map_output_value(&s) {
            Some(inum) => Ok(Item::Value(inum)),
            None => Err(CoreError::SerializationError(format!(
                "Value '{}' not found in enum '{}'",
                s,
                dbt.identifier().name()
            ))),
        },

        val => Err(CoreError::SerializationError(format!(
            "Attempted to serialize non-enum-compatible value '{}' for enum '{}'",
            val,
            dbt.identifier().name()
        ))),
    }
}

fn unwrap_prisma_value(pv: PrismaValue) -> Vec<PrismaValue> {
    match pv {
        PrismaValue::List(l) => l,
        _ => panic!("Invariant error: Called unwrap list value on non-list."),
    }
}
