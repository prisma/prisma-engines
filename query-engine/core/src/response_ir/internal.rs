use super::*;
use crate::{
    constants::custom_types,
    protocol::EngineProtocol,
    result_ast::{RecordSelectionWithRelations, RelationRecordSelection},
    CoreError, QueryResult, RecordAggregations, RecordSelection,
};
use connector::AggregationResult;
use indexmap::IndexMap;
use query_structure::{CompositeFieldRef, Field, Model, PrismaValue, SelectionResult, VirtualSelection};
use schema::{
    constants::{aggregations::*, output_fields::*},
    *,
};
use std::collections::{HashMap, HashSet};

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
pub(crate) fn serialize_internal(
    result: QueryResult,
    field: &OutputField<'_>,
    is_list: bool,
    query_schema: &QuerySchema,
) -> crate::Result<CheckedItemsWithParents> {
    match result {
        QueryResult::RecordSelection(Some(rs)) => {
            serialize_record_selection(*rs, field, field.field_type(), is_list, query_schema)
        }
        QueryResult::RecordSelectionWithRelations(rs) => {
            serialize_record_selection_with_relations(*rs, field, field.field_type(), is_list)
        }
        QueryResult::RecordAggregations(ras) => serialize_aggregations(field, ras),
        QueryResult::Count(c) => {
            // Todo needs a real implementation or needs to move to RecordAggregation
            let mut map: Map = IndexMap::with_capacity(1);
            let mut result = CheckedItemsWithParents::new();

            map.insert(AFFECTED_COUNT.into(), Item::Value(PrismaValue::Int(c as i64)));
            result.insert(None, Item::Map(map));

            Ok(result)
        }
        QueryResult::RawJson(_) => unimplemented!(),
        QueryResult::Id(_) | QueryResult::RecordSelection(None) => unreachable!(),
        QueryResult::Unit => unimplemented!(),
    }
}

fn serialize_aggregations(
    output_field: &OutputField<'_>,
    record_aggregations: RecordAggregations,
) -> crate::Result<CheckedItemsWithParents> {
    let ordering = record_aggregations.selection_order;
    let aggregate_object_type = extract_aggregate_object_type(output_field.field_type());

    let mut results = vec![];

    for row in record_aggregations.results {
        let mut flattened = HashMap::with_capacity(ordering.len());

        for result in row {
            match result {
                AggregationResult::Field(field, value) => {
                    let output_field = aggregate_object_type.find_field(field.name()).unwrap();
                    flattened.insert(field.name().to_owned(), serialize_scalar(output_field, value)?);
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
                        find_nested_aggregate_output_field(aggregate_object_type, UNDERSCORE_AVG, field.name());
                    flattened.insert(format!("_avg_{}", field.name()), serialize_scalar(output_field, value)?);
                }

                AggregationResult::Sum(field, value) => {
                    let output_field =
                        find_nested_aggregate_output_field(aggregate_object_type, UNDERSCORE_SUM, field.name());
                    flattened.insert(format!("_sum_{}", field.name()), serialize_scalar(output_field, value)?);
                }

                AggregationResult::Min(field, value) => {
                    let output_field =
                        find_nested_aggregate_output_field(aggregate_object_type, UNDERSCORE_MIN, field.name());
                    flattened.insert(
                        format!("_min_{}", field.name()),
                        serialize_scalar(output_field, coerce_non_numeric(value, output_field.field_type()))?,
                    );
                }

                AggregationResult::Max(field, value) => {
                    let output_field =
                        find_nested_aggregate_output_field(aggregate_object_type, UNDERSCORE_MAX, field.name());
                    flattened.insert(
                        format!("_max_{}", field.name()),
                        serialize_scalar(output_field, coerce_non_numeric(value, output_field.field_type()))?,
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

    match output_field.field_type() {
        t if t.is_list() => {
            envelope.insert(None, Item::List(results.into()));
        }
        t if t.is_object() => {
            if let Some(item) = results.pop() {
                envelope.insert(None, item);
            };
        }
        _ => unreachable!(),
    };

    Ok(envelope)
}

fn extract_aggregate_object_type<'a, 'b>(output_type: &'b OutputType<'a>) -> &'b ObjectType<'a> {
    match &output_type.inner {
        InnerOutputType::Object(obj) => obj,
        _ => unreachable!("Aggregate output must be a list or an object."),
    }
}

// Workaround until we streamline serialization.
fn find_nested_aggregate_output_field<'a, 'b>(
    object_type: &'b ObjectType<'a>,
    nested_obj_name: &str,
    nested_field_name: &str,
) -> &'b OutputField<'a> {
    let nested_field = object_type.find_field(nested_obj_name).unwrap();
    let nested_object_type = match &nested_field.field_type().inner {
        InnerOutputType::Object(obj) => obj,
        _ => unreachable!("{} output must be an object.", nested_obj_name),
    };

    nested_object_type.find_field(nested_field_name).unwrap()
}

fn coerce_non_numeric(value: PrismaValue, output: &OutputType<'_>) -> PrismaValue {
    match (value, &output.inner) {
        (PrismaValue::Int(0), InnerOutputType::Scalar(ScalarType::String)) => PrismaValue::Null,
        (x, _) => x,
    }
}

fn serialize_record_selection_with_relations(
    record_selection: RecordSelectionWithRelations,
    field: &OutputField<'_>,
    typ: &OutputType<'_>, // We additionally pass the type to allow recursing into nested type definitions of a field.
    is_list: bool,
) -> crate::Result<CheckedItemsWithParents> {
    let name = record_selection.name.clone();

    match &typ.inner {
        inner if typ.is_list() => serialize_record_selection_with_relations(
            record_selection,
            field,
            &OutputType::non_list(inner.clone()),
            true,
        ),
        InnerOutputType::Object(obj) => {
            let result = serialize_objects_with_relation(record_selection, obj)?;

            finalize_objects(field, is_list, result, name)
        }
        // We always serialize record selections into objects or lists on the top levels. Scalars and enums are handled separately.
        _ => unreachable!(),
    }
}

fn serialize_record_selection(
    record_selection: RecordSelection,
    field: &OutputField<'_>,
    typ: &OutputType<'_>, // We additionally pass the type to allow recursing into nested type definitions of a field.
    is_list: bool,
    query_schema: &QuerySchema,
) -> crate::Result<CheckedItemsWithParents> {
    let name = record_selection.name.clone();

    match &typ.inner {
        inner if typ.is_list() => serialize_record_selection(
            record_selection,
            field,
            &OutputType::non_list(inner.clone()),
            true,
            query_schema,
        ),
        InnerOutputType::Object(obj) => {
            let result = serialize_objects(record_selection, obj, query_schema)?;

            finalize_objects(field, is_list, result, name)
        }

        _ => unreachable!(), // We always serialize record selections into objects or lists on the top levels. Scalars and enums are handled separately.
    }
}

fn finalize_objects(
    field: &OutputField<'_>,
    is_list: bool,
    result: IndexMap<Option<SelectionResult>, Vec<Item>>,
    name: String,
) -> Result<IndexMap<Option<SelectionResult>, Item>, CoreError> {
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

enum SerializedFieldWithRelations<'a, 'b> {
    Model(Field, &'a OutputField<'b>),
    VirtualsGroup(&'a str, Vec<&'a VirtualSelection>),
}

impl<'a, 'b> SerializedFieldWithRelations<'a, 'b> {
    fn name(&self) -> &str {
        match self {
            Self::Model(f, _) => f.name(),
            Self::VirtualsGroup(name, _) => name,
        }
    }
}

// TODO: Handle errors properly
fn serialize_objects_with_relation(
    result: RecordSelectionWithRelations,
    typ: &ObjectType<'_>,
) -> crate::Result<UncheckedItemsWithParents> {
    let mut object_mapping = UncheckedItemsWithParents::with_capacity(result.records.records.len());

    let nested = result.nested;

    let fields =
        collect_serialized_fields_with_relations(typ, &result.model, &result.virtuals, &result.records.field_names);

    // Hack: we convert it to a hashset to support contains with &str as input
    // because Vec<String>::contains(&str) doesn't work and we don't want to allocate a string record value
    let selected_db_field_names: HashSet<String> = result.fields.clone().into_iter().collect();

    for record in result.records.records.into_iter() {
        if !object_mapping.contains_key(&record.parent_id) {
            object_mapping.insert(record.parent_id.clone(), Vec::new());
        }

        let values = record.values;
        let mut object = HashMap::with_capacity(values.len());

        for (val, field) in values.into_iter().zip(fields.iter()) {
            // Skip fields that aren't part of the selection set
            if !selected_db_field_names.contains(field.name()) {
                continue;
            }

            match field {
                SerializedFieldWithRelations::Model(Field::Scalar(_), out_field)
                    if !out_field.field_type().is_object() =>
                {
                    object.insert(field.name().to_owned(), serialize_scalar(out_field, val)?);
                }

                SerializedFieldWithRelations::Model(Field::Relation(_), out_field)
                    if out_field.field_type().is_list() =>
                {
                    let inner_typ = out_field.field_type.as_object_type().unwrap();
                    let rrs = nested.iter().find(|rrs| rrs.name == field.name()).unwrap();

                    let items = val
                        .into_list()
                        .unwrap()
                        .into_iter()
                        .map(|value| serialize_relation_selection(rrs, value, inner_typ))
                        .collect::<crate::Result<Vec<_>>>()?;

                    object.insert(field.name().to_owned(), Item::list(items));
                }

                SerializedFieldWithRelations::Model(Field::Relation(_), out_field) => {
                    let inner_typ = out_field.field_type.as_object_type().unwrap();
                    let rrs = nested.iter().find(|rrs| rrs.name == field.name()).unwrap();

                    object.insert(
                        field.name().to_owned(),
                        serialize_relation_selection(rrs, val, inner_typ)?,
                    );
                }

                SerializedFieldWithRelations::VirtualsGroup(group_name, virtuals) => {
                    object.insert(group_name.to_string(), serialize_virtuals_group(val, virtuals)?);
                }

                _ => panic!("unexpected field"),
            }
        }

        let map = reorder_object_with_selection_order(&result.fields, object);

        let result = Item::Map(map);

        object_mapping.get_mut(&record.parent_id).unwrap().push(result);
    }

    Ok(object_mapping)
}

fn serialize_relation_selection(
    rrs: &RelationRecordSelection,
    value: PrismaValue,
    // parent_id: Option<SelectionResult>,
    typ: &ObjectType<'_>,
) -> crate::Result<Item> {
    if value.is_null() {
        return Ok(Item::Value(PrismaValue::Null));
    }

    let mut map = Map::new();

    // TODO: better handle errors
    let mut value_obj: HashMap<String, PrismaValue> = HashMap::from_iter(value.into_object().unwrap());

    let fields = collect_serialized_fields_with_relations(typ, &rrs.model, &rrs.virtuals, &rrs.fields);

    for field in fields {
        let value = value_obj.remove(field.name()).unwrap();

        match field {
            SerializedFieldWithRelations::Model(Field::Scalar(_), out_field) if !out_field.field_type().is_object() => {
                map.insert(field.name().to_owned(), serialize_scalar(out_field, value)?);
            }

            SerializedFieldWithRelations::Model(Field::Relation(_), out_field) if out_field.field_type().is_list() => {
                let inner_typ = out_field.field_type.as_object_type().unwrap();
                let inner_rrs = rrs.nested.iter().find(|rrs| rrs.name == field.name()).unwrap();

                let items = value
                    .into_list()
                    .unwrap()
                    .into_iter()
                    .map(|value| serialize_relation_selection(inner_rrs, value, inner_typ))
                    .collect::<crate::Result<Vec<_>>>()?;

                map.insert(field.name().to_owned(), Item::list(items));
            }

            SerializedFieldWithRelations::Model(Field::Relation(_), out_field) => {
                let inner_typ = out_field.field_type.as_object_type().unwrap();
                let inner_rrs = rrs.nested.iter().find(|rrs| rrs.name == field.name()).unwrap();

                map.insert(
                    field.name().to_owned(),
                    serialize_relation_selection(inner_rrs, value, inner_typ)?,
                );
            }

            SerializedFieldWithRelations::VirtualsGroup(group_name, virtuals) => {
                map.insert(group_name.to_string(), serialize_virtuals_group(value, &virtuals)?);
            }

            _ => (),
        }
    }

    Ok(Item::Map(map))
}

fn collect_serialized_fields_with_relations<'a, 'b>(
    object_type: &'a ObjectType<'b>,
    model: &Model,
    virtuals: &'a [VirtualSelection],
    db_field_names: &'a [String],
) -> Vec<SerializedFieldWithRelations<'a, 'b>> {
    db_field_names
        .iter()
        .map(|name| {
            model
                .fields()
                .all()
                .find(|field| field.name() == name)
                .and_then(|field| {
                    object_type
                        .find_field(field.name())
                        .map(|out_field| SerializedFieldWithRelations::Model(field, out_field))
                })
                .unwrap_or_else(|| {
                    let matching_virtuals = virtuals.iter().filter(|vs| vs.serialized_name().0 == name).collect();
                    SerializedFieldWithRelations::VirtualsGroup(name.as_str(), matching_virtuals)
                })
        })
        .collect()
}

fn serialize_virtuals_group(obj_value: PrismaValue, virtuals: &[&VirtualSelection]) -> crate::Result<Item> {
    let mut db_object: HashMap<String, PrismaValue> = HashMap::from_iter(obj_value.into_object().unwrap());
    let mut out_object = Map::new();

    // We have to reorder the object fields according to selection even if the query
    // builder respects the initial order because JSONB does not preserve order.
    for vs in virtuals {
        let (group_name, nested_name) = vs.serialized_name();

        let value = db_object.remove(nested_name).ok_or_else(|| {
            CoreError::SerializationError(format!(
                "Expected virtual field {nested_name} not found in {group_name} object"
            ))
        })?;

        out_object.insert(nested_name.into(), Item::Value(vs.coerce_value(value)?));
    }

    Ok(Item::Map(out_object))
}

enum SerializedField<'a, 'b> {
    Model(Field, &'a OutputField<'b>),
    Virtual(&'a VirtualSelection),
}

/// Serializes the given result into objects of given type.
/// Doesn't validate the shape of the result set ("unchecked" result).
/// Returns a vector of serialized objects (as Item::Map), grouped into a map by parent, if present.
fn serialize_objects(
    mut result: RecordSelection,
    typ: &ObjectType<'_>,
    query_schema: &QuerySchema,
) -> crate::Result<UncheckedItemsWithParents> {
    // The way our query execution works, we only need to look at nested + lists if we hit an object.
    // Move nested out of result for separate processing.
    let nested = std::mem::take(&mut result.nested);

    // { <nested field name> -> { parent ID -> items } }
    let mut nested_mapping: HashMap<String, CheckedItemsWithParents> =
        process_nested_results(nested, typ, query_schema)?;

    // We need the Arcs to solve the issue where we have multiple parents claiming the same data (we want to move the data out of the nested structure
    // to prevent expensive copying during serialization).

    // Finally, serialize the objects based on the selected fields.
    let mut object_mapping = UncheckedItemsWithParents::with_capacity(result.records.records.len());
    let db_field_names = result.records.field_names;
    let model = result.model;

    let fields: Vec<_> = db_field_names
        .iter()
        .map(|name| {
            model
                .fields()
                .find_from_non_virtual_by_db_name(name)
                .ok()
                .and_then(|field| {
                    typ.find_field(field.name())
                        .map(|out_field| SerializedField::Model(field, out_field))
                })
                .or_else(|| {
                    result
                        .virtual_fields
                        .iter()
                        .find(|f| f.db_alias() == *name)
                        .map(SerializedField::Virtual)
                })
                // Shouldn't happen, implies that the query returned unknown fields.
                .expect("Field must be a known scalar or virtual")
        })
        .collect();

    // Write all fields, nested and list fields unordered into a map, afterwards order all into the final order.
    // If nothing is written to the object, write null instead.
    for record in result.records.records {
        let record_id =
            Some(record.extract_selection_result_from_db_name(&db_field_names, &model.primary_identifier())?);

        if !object_mapping.contains_key(&record.parent_id) {
            object_mapping.insert(record.parent_id.clone(), Vec::new());
        }

        // Write scalars and composites, but skip objects (relations) and scalar lists, which while they are in the selection, are handled separately.
        let values = record.values;
        let mut object = HashMap::with_capacity(values.len());

        for (val, field) in values.into_iter().zip(fields.iter()) {
            match field {
                SerializedField::Model(field, out_field) => {
                    if let Field::Composite(cf) = field {
                        object.insert(field.name().to_owned(), serialize_composite(cf, out_field, val)?);
                    } else if !out_field.field_type().is_object() {
                        object.insert(field.name().to_owned(), serialize_scalar(out_field, val)?);
                    }
                }

                SerializedField::Virtual(vs) => {
                    let (virtual_obj_name, nested_field_name) = vs.serialized_name();

                    let virtual_obj = object
                        .entry(virtual_obj_name.into())
                        .or_insert(Item::Map(Map::new()))
                        .as_map_mut()
                        .expect("Virtual and scalar fields must not collide");

                    virtual_obj.insert(nested_field_name.into(), Item::Value(vs.coerce_value(val)?));
                }
            }
        }

        // Write nested results
        write_nested_items(&record_id, &mut nested_mapping, &mut object, typ)?;

        let map = reorder_object_with_selection_order(&result.fields, object);

        object_mapping.get_mut(&record.parent_id).unwrap().push(Item::Map(map));
    }

    Ok(object_mapping)
}

fn reorder_object_with_selection_order(
    selection_order: &[String],
    mut object: HashMap<String, Item>,
) -> IndexMap<String, Item> {
    selection_order
        .iter()
        .fold(Map::with_capacity(selection_order.len()), |mut acc, field_name| {
            acc.insert(field_name.to_owned(), object.remove(field_name).unwrap());
            acc
        })
}

/// Unwraps are safe due to query validation.
fn write_nested_items(
    record_id: &Option<SelectionResult>,
    items_with_parent: &mut HashMap<String, CheckedItemsWithParents>,
    into: &mut HashMap<String, Item>,
    enclosing_type: &ObjectType<'_>,
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
                let default = match field.field_type() {
                    t if t.is_list() => Item::list(Vec::new()),
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
    enclosing_type: &ObjectType<'_>,
    query_schema: &QuerySchema,
) -> crate::Result<HashMap<String, CheckedItemsWithParents>> {
    // For each nested selected field we need to map the parents to their items.
    let mut nested_mapping = HashMap::with_capacity(nested.len());

    // Parse and validate all nested objects with their respective output type.
    // Unwraps are safe due to query validation.
    for nested_result in nested {
        // todo Workaround, tb changed with flat reads.
        if let QueryResult::RecordSelection(Some(ref rs)) = nested_result {
            let name = rs.name.clone();
            let field = enclosing_type.find_field(&name).unwrap();
            let result = serialize_internal(nested_result, field, false, query_schema)?;

            nested_mapping.insert(name, result);
        }
    }

    Ok(nested_mapping)
}

// Problem: order of selections
fn serialize_composite(cf: &CompositeFieldRef, out_field: &OutputField<'_>, value: PrismaValue) -> crate::Result<Item> {
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
                .field_type()
                .as_object_type()
                .expect("Composite output field is not an object.");

            let composite_type = cf.typ();

            for (field_name, value) in pairs {
                // The field on the composite type.
                // This will cause clashes if one field has an @map("name") and the other field is named "field" directly.
                let inner_field = composite_type
                    .find_field(&field_name)
                    .or_else(|| composite_type.find_field_by_db_name(&field_name))
                    .unwrap();

                // The field on the output object type. Used for the actual serialization process.
                let inner_out_field = object_type.find_field(inner_field.name()).unwrap();

                match &inner_field {
                    Field::Composite(cf) => {
                        map.insert(
                            inner_field.name().to_owned(),
                            serialize_composite(cf, inner_out_field, value)?,
                        );
                    }

                    _ if !inner_out_field.field_type().is_object() => {
                        map.insert(inner_field.name().to_owned(), serialize_scalar(inner_out_field, value)?);
                    }

                    _ => (),
                }
            }

            Ok(Item::Map(map))
        }

        val => Err(CoreError::SerializationError(format!(
            "Attempted to serialize '{}' with non-composite compatible type '{:?}' for field {}.",
            val,
            cf.typ().name(),
            cf.name()
        ))),
    }
}

fn serialize_scalar(field: &OutputField<'_>, value: PrismaValue) -> crate::Result<Item> {
    let field_type = field.field_type();
    match (&value, &field_type.inner) {
        (PrismaValue::Null, _) if field.is_nullable => Ok(Item::Value(PrismaValue::Null)),
        (PrismaValue::List(_), arc_type) if field_type.is_list() => match arc_type {
            InnerOutputType::Scalar(subtype) => {
                let items = unwrap_prisma_value(value)
                    .into_iter()
                    .map(|v| convert_prisma_value(field, v, subtype))
                    .map(|pv| pv.map(Item::Value))
                    .collect::<Result<Vec<Item>, CoreError>>()?;
                Ok(Item::list(items))
            }
            InnerOutputType::Enum(et) => {
                let items = unwrap_prisma_value(value)
                    .into_iter()
                    .map(|v| match &et {
                        EnumType::Database(ref dbt) => convert_enum(v, dbt),
                        _ => unreachable!(),
                    })
                    .collect::<Result<Vec<Item>, CoreError>>()?;
                Ok(Item::list(items))
            }
            _ => Err(CoreError::SerializationError(format!(
                "Attempted to serialize scalar list which contained non-scalar items of type '{:?}' for field {}.",
                arc_type,
                field.name()
            ))),
        },
        (_, InnerOutputType::Enum(et)) => match &et {
            EnumType::Database(ref db) => convert_enum(value, db),
            _ => unreachable!(),
        },
        (_, InnerOutputType::Scalar(st)) => Ok(Item::Value(convert_prisma_value(field, value, st)?)),
        (pv, ot) => Err(CoreError::SerializationError(format!(
            "Attempted to serialize scalar '{}' with non-scalar compatible type '{:?}' for field {}.",
            pv,
            ot,
            field.name()
        ))),
    }
}

fn convert_prisma_value(field: &OutputField<'_>, value: PrismaValue, st: &ScalarType) -> crate::Result<PrismaValue> {
    match crate::executor::get_engine_protocol() {
        #[cfg(feature = "graphql-protocol")]
        EngineProtocol::Graphql => convert_prisma_value_graphql_protocol(field, value, st),
        EngineProtocol::Json => convert_prisma_value_json_protocol(field, value, st),
    }
}

#[cfg(feature = "graphql-protocol")]
fn convert_prisma_value_graphql_protocol(
    field: &OutputField<'_>,
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
        (ScalarType::Geometry, PrismaValue::Json(s)) => PrismaValue::Json(s),

        // The Decimal type doesn't have a corresponding PrismaValue variant. We need to serialize it
        // to String so that client can deserialize it as Decimal again.
        (ScalarType::Decimal, PrismaValue::Int(i)) => PrismaValue::String(i.to_string()),
        (ScalarType::Decimal, PrismaValue::Float(f)) => PrismaValue::String(f.to_string()),

        (st, pv) => {
            return Err(crate::FieldConversionError::create(
                field.name().clone().into_owned(),
                format!("{st:?}"),
                pv.to_string(),
            ))
        }
    };

    Ok(item_value)
}

/// Since the JSON protocol is "schema-less" by design, clients require type information for them to
/// properly deserialize special values such as bytes, decimal, datetime, etc.
fn convert_prisma_value_json_protocol(
    field: &OutputField<'_>,
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
        (ScalarType::Geometry, PrismaValue::Json(x)) => {
            custom_types::make_object(custom_types::JSON, PrismaValue::Json(x))
        }

        // Identity matchers
        (ScalarType::String, PrismaValue::String(x)) => PrismaValue::String(x),
        (ScalarType::UUID, PrismaValue::Uuid(x)) => PrismaValue::Uuid(x),
        (ScalarType::Boolean, PrismaValue::Boolean(x)) => PrismaValue::Boolean(x),
        (ScalarType::Int, PrismaValue::Int(x)) => PrismaValue::Int(x),
        (ScalarType::Float, PrismaValue::Float(x)) => PrismaValue::Float(x),

        (st, pv) => {
            return Err(crate::FieldConversionError::create(
                field.name().clone().into_owned(),
                format!("{st:?}"),
                pv.to_string(),
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
