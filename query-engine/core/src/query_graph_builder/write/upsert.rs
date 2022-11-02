use super::{write_args_parser::WriteArgsParser, *};
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, QueryGraph, QueryGraphDependency},
    ParsedField, ParsedInputMap, ParsedInputValue,
};
use connector::IntoFilter;
use prisma_models::ModelRef;
use schema::ConnectorContext;
use std::sync::Arc;

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
pub fn upsert_record(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    model: ModelRef,
    mut field: ParsedField,
) -> QueryGraphBuilderResult<()> {
    let where_argument = field.where_arg()?.unwrap();
    let create_argument = field.create_arg()?.unwrap();
    let update_argument = field.update_arg()?.unwrap();

    let can_use_native_upsert = can_use_connector_native_upsert(
        &model,
        &where_argument,
        &create_argument,
        &update_argument,
        connector_ctx,
    );

    let filter = extract_unique_filter(where_argument, &model)?;
    let read_query = read::find_unique(field.clone(), model.clone())?;

    if can_use_native_upsert {
        if let ReadQuery::RecordQuery(read) = read_query {
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
                read,
            ));
            return Ok(());
        }
    }

    graph.flag_transactional();

    let model_id = model.primary_identifier();

    let read_parent_records = utils::read_ids_infallible(model.clone(), model_id.clone(), filter.clone());
    let read_parent_records_node = graph.create_node(read_parent_records);

    let create_node = create::create_record_node(graph, connector_ctx, Arc::clone(&model), create_argument)?;

    let update_node = update::update_record_node(graph, connector_ctx, filter, Arc::clone(&model), update_argument)?;

    let read_node_create = graph.create_node(Query::Read(read_query.clone()));
    let read_node_update = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node_create);
    graph.add_result_node(&read_node_update);

    let if_node = graph.create_node(Flow::default_if());

    graph.create_edge(
        &read_parent_records_node,
        &if_node,
        QueryGraphDependency::ProjectedDataDependency(
            model_id.clone(),
            Box::new(|if_node, parent_ids| {
                if let Node::Flow(Flow::If(_)) = if_node {
                    // Todo: This looks super unnecessary
                    Ok(Node::Flow(Flow::If(Box::new(move || !parent_ids.is_empty()))))
                } else {
                    Ok(if_node)
                }
            }),
        ),
    )?;

    // In case the connector doesn't support referential integrity, we add a subtree to the graph that emulates the ON_UPDATE referential action.
    // When that's the case, we create an intermediary node to which we connect all the nodes reponsible for emulating the referential action
    // Then, we connect the if node to that intermediary emulation node. This enables performing the emulation only in case the graph traverses
    // the update path (if the children already exists and goes to the THEN node).
    // It's only after we've executed the emulation that it'll traverse the update node, hence the ExecutionOrder between
    // the emulation node and the update node.
    if let Some(emulation_node) = utils::insert_emulated_on_update_with_intermediary_node(
        graph,
        connector_ctx,
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
    graph.create_edge(
        &update_node,
        &read_node_update,
        QueryGraphDependency::ProjectedDataDependency(
            model_id.clone(),
            Box::new(move |mut read_node_update, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(
                        "Expected a valid parent ID to be present for create follow-up for upsert query.".to_string(),
                    )),
                }?;

                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = read_node_update {
                    rq.add_filter(parent_id.filter());
                };

                Ok(read_node_update)
            }),
        ),
    )?;

    graph.create_edge(
        &create_node,
        &read_node_create,
        QueryGraphDependency::ProjectedDataDependency(
            model_id,
            Box::new(move |mut read_node_create, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(
                        "Expected a valid parent ID to be present for update follow-up for upsert query.".to_string(),
                    )),
                }?;

                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = read_node_create {
                    rq.add_filter(parent_id.filter());
                };

                Ok(read_node_create)
            }),
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
fn can_use_connector_native_upsert(
    model: &ModelRef,
    where_field: &ParsedInputMap,
    create_argument: &ParsedInputMap,
    update_argument: &ParsedInputMap,
    connector_ctx: &ConnectorContext,
) -> bool {
    let has_nested_create = create_argument
        .iter()
        .any(|(field_name, _)| model.fields().find_from_relation_fields(&field_name).is_ok());

    let has_nested_update = update_argument
        .iter()
        .any(|(field_name, _)| model.fields().find_from_relation_fields(&field_name).is_ok());

    let empty_update = update_argument.iter().len() == 0;

    let has_one_unique = where_field
        .iter()
        .filter(|(field_name, _)| is_unique_field(field_name, model))
        .count()
        == 1;

    let where_values_same_as_create = where_field
        .iter()
        .all(|(field_name, input)| where_and_create_equal(&field_name, &input, &create_argument));

    connector_ctx.can_native_upsert()
        && has_one_unique
        && !has_nested_create
        && !has_nested_update
        && !empty_update
        && where_values_same_as_create
}

fn is_unique_field(field_name: &String, model: &ModelRef) -> bool {
    match model.fields().find_from_scalar(&field_name) {
        Ok(field) => field.unique(),
        Err(_) => resolve_compound_field(field_name, model).is_some(),
    }
}

/// Make sure the unique fields defined in the where clause have the same values
/// as in the create of the upsert.
fn where_and_create_equal(field_name: &str, where_value: &ParsedInputValue, create_map: &ParsedInputMap) -> bool {
    match where_value {
        ParsedInputValue::Map(inner_map) => inner_map
            .iter()
            .all(|(inner_field, inner_value)| where_and_create_equal(inner_field, inner_value, create_map)),
        _ => Some(where_value) == create_map.get(field_name),
    }
}
