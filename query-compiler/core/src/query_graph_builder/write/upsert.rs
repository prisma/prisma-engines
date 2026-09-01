use super::{write_args_parser::WriteArgsParser, *};
use crate::{
    DataExpectation, ParsedField, ParsedInputMap, ParsedInputValue, ParsedObject, RowSink,
    inputs::{IfInput, RecordQueryFilterInput, UpdateRecordSelectorsInput},
    query_ast::*,
    query_graph::{Flow, QueryGraph, QueryGraphDependency},
};
use query_structure::{Model, ScalarFieldRef};
use schema::{QuerySchema, compound_index_field_name};

/// Handles a top-level upsert
///
/// ```text
///                         ┌─────────────────┐           ┌ ─ ─ ─ ─ ─ ─
///                         │   Read Parent   │─ ─ ─ ─ ─ ▶    Result   │
///                         └─────────────────┘           └ ─ ─ ─ ─ ─ ─
///                                  │
///                                  │
///                                  │
///                                  │
///                                  ▼
///                         ┌─────────────────┐
///           ┌───Then──────│   If (exists)   │──Else─────┐
///           │             └─────────────────┘           │
///           │                                           │
/// ┌ ─ ─ ─ ─ ▼ ─ ─ ─ ─ ┐                                 │
///  ┌─────────────────┐                                  │
/// ││    Join Node    ││                                 │
///  └─────────────────┘                                  ▼
/// │         │         │                        ┌─────────────────┐
///           │                                  │  Create Parent  │
/// │         ▼         │                        └─────────────────┘
///  ┌─────────────────┐                                  │
/// ││ Insert onUpdate ││                                 │
///  │emulation subtree│                                  │
/// ││for all relations││                                 │
///  │ pointing to the │                                  ▼
/// ││  Parent model   ││                        ┌─────────────────┐
///  └─────────────────┘                         │   Read Parent   │
/// └ ─ ─ ─ ─ ┬ ─ ─ ─ ─ ┘                        └─────────────────┘
///           │
///           │
///           ▼
///  ┌─────────────────┐
///  │  Update Parent  │
///  └─────────────────┘
///           │
///           ▼
///  ┌─────────────────┐
///  │   Read Parent   │
///  └─────────────────┘
/// ```
pub(crate) fn upsert_record(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model: Model,
    mut field: ParsedField<'_>,
) -> QueryGraphBuilderResult<()> {
    let where_argument = field.where_arg()?.unwrap();
    let create_argument = field.create_arg()?.unwrap();
    let update_argument = field.update_arg()?.unwrap();
    let selection = &field.nested_fields;

    let can_use_native_upsert = can_use_connector_native_upsert(
        &model,
        &where_argument,
        &create_argument,
        &update_argument,
        selection,
        query_schema,
    );

    // The native upsert must additionally arbitrate on a constraint the database
    // can infer, and it has to be the very one the `where` clause named — see
    // `conflict_target`.
    let conflict_columns = can_use_native_upsert
        .then(|| unique_selector(&where_argument, &model))
        .flatten()
        .and_then(|selector| conflict_target(&model, selector));

    let filter = extract_unique_filter(where_argument, &model)?;
    let read_query = read::find_unique(field.clone(), model.clone(), query_schema)?;

    if let Some(conflict_columns) = conflict_columns
        && let ReadQuery::RecordQuery(read) = read_query
    {
        let mut create_write_args = WriteArgsParser::from(&model, create_argument)?.args;
        let mut update_write_args = WriteArgsParser::from(&model, update_argument)?.args;

        create_write_args.add_datetimes(&model);
        update_write_args.add_datetimes(&model);

        graph.create_node(WriteQuery::native_upsert(
            field.name,
            model,
            filter.into(),
            create_write_args,
            update_write_args,
            conflict_columns,
            read,
        ));

        return Ok(());
    }

    graph.flag_transactional();

    let model_id = model.shard_aware_primary_identifier();

    let read_parent_records = utils::read_ids_infallible(model.clone(), model_id.clone(), filter.clone());
    let read_parent_records_node = graph.create_node(read_parent_records);

    let create_node = create::create_record_node(graph, query_schema, model.clone(), create_argument)?;

    let update_node = update::update_record_node(
        graph,
        query_schema,
        filter,
        model.clone(),
        update_argument,
        Some(&field),
    )?;

    let read_node_create = graph.create_node(Query::Read(read_query.clone()));
    let read_node_update = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node_create);
    graph.add_result_node(&read_node_update);

    let if_node = graph.create_node(Flow::if_non_empty());

    graph.create_edge(
        &read_parent_records_node,
        &if_node,
        QueryGraphDependency::ProjectedDataDependency(model_id.clone(), RowSink::All(&IfInput), None),
    )?;

    // In case the connector doesn't support referential integrity, we add a subtree to the graph that emulates the ON_UPDATE referential action.
    // When that's the case, we create an intermediary node to which we connect all the nodes reponsible for emulating the referential action
    // Then, we connect the if node to that intermediary emulation node. This enables performing the emulation only in case the graph traverses
    // the update path (if the children already exists and goes to the THEN node).
    // It's only after we've executed the emulation that it'll traverse the update node, hence the ExecutionOrder between
    // the emulation node and the update node.
    if let Some(emulation_node) = utils::insert_emulated_on_update_with_intermediary_node(
        graph,
        query_schema,
        &model,
        &read_parent_records_node,
        &update_node,
    )? {
        graph.create_edge(&if_node, &emulation_node, QueryGraphDependency::Then)?;
        graph.create_edge(&emulation_node, &update_node, QueryGraphDependency::ExecutionOrder)?;
    } else {
        graph.create_edge(&if_node, &update_node, QueryGraphDependency::Then)?;
    }

    graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;

    // Pass-in the read parent record result to the update node RecordFilter to avoid a redundant read.
    graph.create_edge(
        &read_parent_records_node,
        &update_node,
        QueryGraphDependency::ProjectedDataDependency(
            model_id.clone(),
            RowSink::ExactlyOne(&UpdateRecordSelectorsInput),
            None,
        ),
    )?;

    graph.create_edge(
        &update_node,
        &read_node_update,
        QueryGraphDependency::ProjectedDataDependency(
            model_id.clone(),
            RowSink::ExactlyOneFilter(&RecordQueryFilterInput),
            Some(DataExpectation::non_empty_rows(
                MissingRecord::builder().operation(DataOperation::Upsert).build(),
            )),
        ),
    )?;

    graph.create_edge(
        &create_node,
        &read_node_create,
        QueryGraphDependency::ProjectedDataDependency(
            model_id,
            RowSink::ExactlyOneFilter(&RecordQueryFilterInput),
            Some(DataExpectation::non_empty_rows(
                MissingRecord::builder().operation(DataOperation::Upsert).build(),
            )),
        ),
    )?;

    Ok(())
}

// This optimisation on our upserts allows us to use the `INSERT ... ON CONFLICT SET ..`
// when the query matches the following conditions:
// 1. The data connector supports it
// 2. The create and update arguments do not have any nested queries
// 3. There is only 1 unique field in the where clause
// 4. The unique field defined in where clause has the same value as defined in the create arguments
//
// The caller checks one further condition: the unique named in the where clause
// must be usable as an `ON CONFLICT` target. See `conflict_target`.
fn can_use_connector_native_upsert<'a>(
    model: &Model,
    where_field: &ParsedInputMap<'a>,
    create_argument: &ParsedInputMap<'a>,
    update_argument: &ParsedInputMap<'a>,
    selection: &Option<ParsedObject<'_>>,
    query_schema: &QuerySchema,
) -> bool {
    let has_nested_selects = has_nested_selects(selection);

    let has_nested_create = create_argument
        .iter()
        .any(|(field_name, _)| model.fields().find_from_relation_fields(field_name).is_ok());

    let has_nested_update = update_argument
        .iter()
        .any(|(field_name, _)| model.fields().find_from_relation_fields(field_name).is_ok());

    let empty_update = update_argument.iter().len() == 0;

    let has_one_unique = where_field
        .iter()
        .filter(|(field_name, _)| is_unique_field(field_name, model))
        .count()
        == 1;

    let where_values_same_as_create = where_field
        .iter()
        .all(|(field_name, input)| where_and_create_equal(field_name, input, create_argument));

    query_schema.can_native_upsert()
        && has_one_unique
        && !has_nested_create
        && !has_nested_update
        && !empty_update
        && !has_nested_selects
        && where_values_same_as_create
        && !query_schema.relation_mode().is_prisma()
}

/// The single `where` key that names a unique constraint.
///
/// `can_use_connector_native_upsert` separately requires that there be exactly
/// one; the remaining keys are non-unique filters that narrow the update.
fn unique_selector<'a>(where_field: &'a ParsedInputMap<'_>, model: &Model) -> Option<&'a str> {
    where_field
        .iter()
        .map(|(field_name, _)| field_name.as_ref())
        .find(|field_name| is_unique_field(field_name, model))
}

/// The columns an `INSERT ... ON CONFLICT` can arbitrate on for the unique that
/// `where` names, or `None` when that unique cannot be a conflict target.
///
/// The arbiter has to be the constraint the `where` clause actually named, not
/// merely one the filter happens to cover. Picking a narrower unique makes the
/// statement conflict on rows the `where` clause does not select: with a
/// `@@unique([email, status])` selector and a total `@unique` on `email` alone,
/// `ON CONFLICT ("email")` collides with a row holding a different `status`,
/// whose `DO UPDATE ... WHERE` then matches nothing, so the upsert quietly
/// returns no row instead of creating one or reporting the unique violation.
///
/// Partial (`WHERE`-filtered) uniques are never usable either. Inferring one
/// requires repeating its predicate in the statement, which the generated SQL
/// does not carry, so PostgreSQL rejects it with 42P10 ("there is no unique or
/// exclusion constraint matching the ON CONFLICT specification") on every call,
/// whatever the data.
///
/// `None` means the connector-native upsert is not usable, and the caller falls
/// back to the read-then-write graph, which handles both cases correctly.
fn conflict_target(model: &Model, selector: &str) -> Option<Vec<ScalarFieldRef>> {
    // A compound selector names the primary key, which is never partial...
    if let Some(primary_key) = resolve_compound_id(selector, model) {
        return Some(primary_key);
    }

    // ...or exactly one `@@unique` index.
    if let Some(index) = model
        .unique_indexes()
        .filter(|index| index.fields().len() > 1)
        .find(|index| compound_index_field_name(*index) == selector)
    {
        return (!index.is_partial()).then(|| {
            index
                .fields()
                .map(|f| ScalarFieldRef::from((model.dm.clone(), f)))
                .collect()
        });
    }

    // A single-column selector is usable when the primary key or a non-partial
    // unique index covers exactly that column.
    let field = model.fields().find_from_scalar(selector).ok()?;
    let is_single_column_id = model.fields().id_fields().is_some_and(|ids| {
        let ids: Vec<_> = ids.collect();
        ids.len() == 1 && ids[0] == field
    });
    let has_total_unique = model.unique_indexes().filter(|index| !index.is_partial()).any(|index| {
        let mut fields = index.fields();
        fields.len() == 1
            && fields
                .next()
                .is_some_and(|f| ScalarFieldRef::from((model.dm.clone(), f)) == field)
    });

    (is_single_column_id || has_total_unique).then(|| vec![field])
}

fn is_unique_field(field_name: &str, model: &Model) -> bool {
    match model.fields().find_from_scalar(field_name) {
        Ok(field) => field.unique(),
        Err(_) => resolve_compound_field(field_name, model).is_some(),
    }
}

fn has_nested_selects(selection: &Option<ParsedObject<'_>>) -> bool {
    if let Some(parsed_object) = selection {
        parsed_object
            .fields
            .iter()
            .any(|field| field.parsed_field.nested_fields.is_some())
    } else {
        false
    }
}

/// Make sure the unique fields defined in the where clause have the same values
/// as in the create of the upsert.
fn where_and_create_equal<'a>(
    field_name: &str,
    where_value: &ParsedInputValue<'a>,
    create_map: &ParsedInputMap<'a>,
) -> bool {
    match where_value {
        ParsedInputValue::Map(inner_map) => inner_map
            .iter()
            .all(|(inner_field, inner_value)| where_and_create_equal(inner_field, inner_value, create_map)),
        _ => Some(where_value) == create_map.get(field_name),
    }
}
