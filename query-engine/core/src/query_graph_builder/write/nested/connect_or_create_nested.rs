use super::*;
use crate::query_graph_builder::write::utils::coerce_vec;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    InputAssertions, ParsedInputMap, ParsedInputValue,
};
use connector::{Filter, IdFilter};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Handles nested connect cases.
///
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn nested_connect_or_create(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();
    let values = utils::coerce_vec(value);

    if relation.is_many_to_many() {
        handle_many_to_many(graph, parent_node, parent_relation_field, values, child_model)
    } else if relation.is_one_to_many() {
        handle_one_to_many(graph, parent_node, parent_relation_field, values, child_model)
    } else {
        handle_one_to_one(graph, parent_node, parent_relation_field, values, child_model)
    }
}

/// Handles a nested connect-or-create many-to-many relation case.
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent       ────────────────────────┐
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │              │
/// │           │                                 │
/// │           │                  │              │
/// │           │                                 │
/// │           ▼                  ▼              │
/// │  ┌─────────────────┐  ┌ ─ ─ ─ ─ ─ ─         │
/// ├──│   Read Child    │      Result   │        │
/// │  └─────────────────┘  └ ─ ─ ─ ─ ─ ─         │
/// │           │                                 │
/// │           │                                 │
/// │           │                                 │
/// │           ▼                                 │
/// │  ┌─────────────────┐                        │
/// │  │   If (exists)   │────────────┐           │
/// │  └─────────────────┘            │           │
/// │           │                     │           │
/// │           │                     │           │
/// │           │                     │           │
/// │           ▼                     ▼           │
/// │  ┌─────────────────┐   ┌─────────────────┐  │
/// └─▶│     Connect     │   │  Create Child   │  │
///    └─────────────────┘   └─────────────────┘  │
///                                   │           │
///                                   │           │
///                                   │           │
///                                   ▼           │
///                          ┌─────────────────┐  │
///                          │     Connect     │◀─┘
///                          └─────────────────┘
/// ```
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in values {
        let mut value: ParsedInputMap = value.try_into()?;

        let where_arg = value.remove("where").unwrap();
        let where_map: ParsedInputMap = where_arg.try_into()?;

        let create_arg = value.remove("create").unwrap();
        let create_map: ParsedInputMap = create_arg.try_into()?;

        let filter = extract_unique_filter(where_map, &child_model)?;
        let read_node = graph.create_node(utils::read_ids_infallible(
            child_model.clone(),
            child_model.primary_identifier(),
            filter,
        ));

        let create_node = create::create_record_node(graph, Arc::clone(child_model), create_map)?;
        let if_node = graph.create_node(Flow::default_if());

        let connect_exists_node =
            connect::connect_records_node(graph, &parent_node, &read_node, &parent_relation_field, 1)?;

        let _connect_create_node =
            connect::connect_records_node(graph, &parent_node, &create_node, &parent_relation_field, 1)?;

        graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;
        graph.create_edge(
            &read_node,
            &if_node,
            QueryGraphDependency::ParentProjection(
                child_model.primary_identifier(),
                Box::new(|if_node, child_ids| {
                    if let Node::Flow(Flow::If(_)) = if_node {
                        Ok(Node::Flow(Flow::If(Box::new(move || !child_ids.is_empty()))))
                    } else {
                        Ok(if_node)
                    }
                }),
            ),
        )?;

        graph.create_edge(&if_node, &connect_exists_node, QueryGraphDependency::Then)?;
        graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;
    }

    Ok(())
}

fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    if parent_relation_field.is_inlined_on_enclosing_model() {
        one_to_many_inlined_parent(graph, parent_node, parent_relation_field, values, child_model)
    } else {
        one_to_many_inlined_child(graph, parent_node, parent_relation_field, values, child_model)
    }
}

/// Handles one-to-many-relation cases where the inlining is done on the child.
/// This implies that the child model is the many side of the relation.
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent       ────────────────────────┐
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │              │
/// │           │                                 │
/// │           │                  │              │
/// │           │                                 │
/// │           ▼                  ▼              │
/// │  ┌─────────────────┐  ┌ ─ ─ ─ ─ ─ ─         │
/// │  │   Read Child    │      Result   │        │
/// │  └─────────────────┘  └ ─ ─ ─ ─ ─ ─         │
/// │           │                                 │
/// │           │                                 │
/// │           │                                 │
/// │           ▼                                 │
/// │  ┌─────────────────┐                        │
/// │  │   If (exists)   │────Else────┐           │
/// │  └─────────────────┘            │           │
/// │           │                     │           │
/// │         Then                    │           │
/// │           │                     │           │
/// │           ▼                     ▼           │
/// │  ┌─────────────────┐   ┌─────────────────┐  │
/// └─▶│  Update Child   │   │  Create Child   │◀─┘
///    └─────────────────┘   └─────────────────┘
/// ```
fn one_to_many_inlined_child(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in values {
        let mut value: ParsedInputMap = value.try_into()?;

        let where_arg = value.remove("where").unwrap();
        let where_map: ParsedInputMap = where_arg.try_into()?;

        let create_arg = value.remove("create").unwrap();
        let create_map: ParsedInputMap = create_arg.try_into()?;

        let filter = extract_unique_filter(where_map, &child_model)?;
        let read_node = graph.create_node(utils::read_ids_infallible(
            child_model.clone(),
            child_model.primary_identifier(),
            filter.clone(),
        ));

        let if_node = graph.create_node(Flow::default_if());
        let update_child_node = utils::update_records_node_placeholder(graph, filter, Arc::clone(child_model));
        let create_node = create::create_record_node(graph, Arc::clone(child_model), create_map)?;

        graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;
        graph.create_edge(&if_node, &update_child_node, QueryGraphDependency::Then)?;
        graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;

        graph.create_edge(
            &read_node,
            &if_node,
            QueryGraphDependency::ParentProjection(
                child_model.primary_identifier(),
                Box::new(|if_node, child_ids| {
                    if let Node::Flow(Flow::If(_)) = if_node {
                        Ok(Node::Flow(Flow::If(Box::new(move || !child_ids.is_empty()))))
                    } else {
                        Ok(if_node)
                    }
                }),
            ),
        )?;

        let parent_link = parent_relation_field.linking_fields();
        let child_link = parent_relation_field.related_field().linking_fields();

        graph.create_edge(
            &parent_node,
            &create_node,
            QueryGraphDependency::ParentProjection(
                parent_link.clone(),
                Box::new(move |mut update_node, mut parent_ids| {
                    let parent_id = match parent_ids.pop() {
                        Some(id) => Ok(id),
                        None => Err(QueryGraphBuilderError::AssertionError(format!(
                            "[Query Graph] Expected a valid parent ID to be present for a nested connect or create."
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_node {
                        wq.inject_projection_into_args(child_link.assimilate(parent_id)?);
                    }

                    Ok(update_node)
                }),
            ),
        )?;

        let child_link = parent_relation_field.related_field().linking_fields();
        graph.create_edge(
            &parent_node,
            &update_child_node,
            QueryGraphDependency::ParentProjection(
                parent_link,
                Box::new(move |mut update_node, mut parent_ids| {
                    let parent_id = match parent_ids.pop() {
                        Some(id) => Ok(id),
                        None => Err(QueryGraphBuilderError::AssertionError(format!(
                            "[Query Graph] Expected a valid parent ID to be present for a nested connect or create."
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_node {
                        wq.inject_projection_into_args(child_link.assimilate(parent_id)?);
                    }

                    Ok(update_node)
                }),
            ),
        )?;
    }

    Ok(())
}

fn one_to_many_inlined_parent(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    todo!()
}

fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    todo!()
}
