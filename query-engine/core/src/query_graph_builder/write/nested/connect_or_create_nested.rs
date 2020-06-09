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

/// Handles nested connect or create cases.
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

/// Dispatcher for one-to-many relations.
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

/// Dispatcher for one-to-one relations.
fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let value = values.pop().unwrap();
    let mut value: ParsedInputMap = value.try_into()?;

    let where_arg = value.remove("where").unwrap();
    let where_map: ParsedInputMap = where_arg.try_into()?;

    let create_arg = value.remove("create").unwrap();
    let create_data: ParsedInputMap = create_arg.try_into()?;

    let filter = extract_unique_filter(where_map, &child_model)?;

    if parent_relation_field.is_inlined_on_enclosing_model() {
        one_to_one_inlined_parent(
            graph,
            parent_node,
            parent_relation_field,
            filter,
            create_data,
            child_model,
        )
    } else {
        one_to_one_inlined_child(
            graph,
            parent_node,
            parent_relation_field,
            filter,
            create_data,
            child_model,
        )
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
        let parent_link = parent_relation_field.linking_fields();
        let child_link = parent_relation_field.related_field().linking_fields();

        let mut value: ParsedInputMap = value.try_into()?;

        let where_arg = value.remove("where").unwrap();
        let where_map: ParsedInputMap = where_arg.try_into()?;

        let create_arg = value.remove("create").unwrap();
        let create_map: ParsedInputMap = create_arg.try_into()?;

        let filter = extract_unique_filter(where_map, &child_model)?;
        let read_node = graph.create_node(utils::read_ids_infallible(
            child_model.clone(),
            child_link.clone(),
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

/// Handles one-to-many-relation cases where the inlining is done on the parent.
/// This implies that the parent model is the many side of the relation, which
/// also implies that there can only be one `value` in `values`.
///
///    ┌─────────────────┐
/// ┌──│   Read Child    │
/// │  └─────────────────┘
/// │           │
/// │           │
/// │           │
/// │           ▼
/// │  ┌─────────────────┐
/// │  │   If (exists)   │──┬────Else───┐
/// │  └─────────────────┘  │           │
/// │           │           │           │
/// │         Then          │           │
/// │           │           │           │
/// │           ▼           │           ▼
/// │  ┌─────────────────┐  │  ┌─────────────────┐
/// ├─▶│   Return Link   │  │  │  Create Child   │
/// │  └─────────────────┘  │  └─────────────────┘
/// │                       │           │
/// │                       │           │
/// │                       │           │
/// │                       │           ▼
/// │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐  │  ┌─────────────────┐
/// └─▶      Parent       ◀─┘  │   Return Link   │
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ┘     └─────────────────┘
///             │
///             ▼
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///          Result
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
fn one_to_many_inlined_parent(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let parent_link = parent_relation_field.linking_fields();
    let child_link = parent_relation_field.related_field().linking_fields();

    let value = values.pop().unwrap();
    let mut value: ParsedInputMap = value.try_into()?;

    let where_arg = value.remove("where").unwrap();
    let where_map: ParsedInputMap = where_arg.try_into()?;

    let create_arg = value.remove("create").unwrap();
    let create_map: ParsedInputMap = create_arg.try_into()?;

    let filter = extract_unique_filter(where_map, &child_model)?;
    let read_node = graph.create_node(utils::read_ids_infallible(
        child_model.clone(),
        child_link.clone(),
        filter.clone(),
    ));

    graph.mark_nodes(&parent_node, &read_node);
    graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;

    let if_node = graph.create_node(Flow::default_if());
    let create_node = create::create_record_node(graph, Arc::clone(child_model), create_map)?;
    let return_existing = graph.create_node(Flow::Return(None));
    let return_create = graph.create_node(Flow::Return(None));

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

    graph.create_edge(&if_node, &return_existing, QueryGraphDependency::Then)?;
    graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;

    graph.create_edge(
        &if_node,
        &parent_node,
        QueryGraphDependency::ParentProjection(
            child_link.clone(),
            Box::new(move |mut parent, mut child_ids| {
                let child_id = child_ids.pop().unwrap();
                if let Node::Query(Query::Write(ref mut wq)) = parent {
                    wq.inject_projection_into_args(parent_link.assimilate(child_id)?);
                }

                Ok(parent)
            }),
        ),
    )?;

    graph.create_edge(
        &read_node,
        &return_existing,
        QueryGraphDependency::ParentProjection(
            child_link.clone(),
            Box::new(move |return_node, child_ids| {
                if let Node::Flow(Flow::Return(_)) = return_node {
                    Ok(Node::Flow(Flow::Return(Some(child_ids))))
                } else {
                    Ok(return_node)
                }
            }),
        ),
    )?;

    graph.create_edge(
        &create_node,
        &return_create,
        QueryGraphDependency::ParentProjection(
            child_link,
            Box::new(move |return_node, child_ids| {
                if let Node::Flow(Flow::Return(_)) = return_node {
                    Ok(Node::Flow(Flow::Return(Some(child_ids))))
                } else {
                    Ok(return_node)
                }
            }),
        ),
    )?;

    Ok(())
}

/// Handles one-to-one relations where the inlining is done on the parent record
/// The resulting graph:
/// ```text
///        ┌────────────────────────┐
///        │       Read Child       │─────────────────────┬────────────────────────────────────────────┐
///        └────────────────────────┘                     │                                            │
///                     │                                 │                                            │
///                     │                                 │                                            │
///                     ▼                                 │                                            │
///        ┌────────────────────────┐                     │                                            │
/// ┌───┬──│      If (exists)       │──────────Then───────┤                                            │
/// │   │  └────────────────────────┘                     │                                            │
/// │   │               │Else                             │                                            │
/// │   │               │                                 │                                            │
/// │   │               ▼                  ┌─── ──── ──── ▼─── ──── ──── ──── ──── ──── ──── ──── ─┐   │
/// │   │  ┌────────────────────────┐      │ ┌────────────────────────┐                            │   │
/// │   │  │      Create Child      │    ┌─┼─│    Read ex. Parent     │──┐                         │   │
/// │   │  └────────────────────────┘    │ │ └────────────────────────┘  │                             │
/// │   │               │                │ │              │              │                         │   │
/// │   │               ▼                │                ▼              │(Fail on p > 0 if parent │   │
/// │   │  ┌────────────────────────┐    │ │ ┌────────────────────────┐  │     side required)      │   │
/// │   │  │      Return Link       │    │ │ │ If p > 0 && p. inlined │  │                         │   │
/// │   │  └────────────────────────┘    │ │ └────────────────────────┘  │                             │
/// │   │                                │ │              │              │                         │   │
/// │   │                                │                ▼              │                         │   │
/// │   │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─     │ │ ┌────────────────────────┐  │                         │   │
/// │   └─▶          Parent         │    │ │ │   Update ex. parent    │◀─┘                         │   │
/// │      └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─     │ │ └────────────────────────┘                      ┌───┐     │
/// │                   │                │ │         then                                    │ 1 │ │   │
/// │   ┌───────────────┘                │                                                   └───┘ │   │
/// │   │               │                │ └─── ──── ──── ──── ──── ──── ──── ──── ──── ──── ──── ─┘   │
/// │   │               ▼                │                                                             │
/// │   │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─     │                                                             │
/// │   │         Read Result       │    │  ┌────────────────────────┐                                 │
/// │   │  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─     └─▶│      Return Link       │◀────────────────────────────────┘
/// │   │                                   └────────────────────────┘
/// │   │
/// │   └────────────────┐
/// │                    │
/// │                    │
/// │     ┌───  ────  ───┼  ────  ────  ────  ────  ────  ────  ──┐
/// │                    ▼                                        │
/// │       ┌────────────────────────┐
/// │   ┌─┼─│     Read ex. child     │──┐
/// │   │ │ └────────────────────────┘  │                         │
/// │   │ │              │              │                         │
/// │   │ │              ▼              │(Fail on c > 0 if child  │
/// │   │   ┌────────────────────────┐  │     side required)      │
/// │   │   │ If c > 0 && c. inlined │  │
/// │   │ │ └────────────────────────┘  │
/// │   │ │         then │              │                         │
/// │   │ │              ▼              │                         │
/// │   │ │ ┌────────────────────────┐  │                         │
/// │   │   │    Update ex. child    │◀─┘                   ┌───┐ │
/// │   │   └────────────────────────┘                      │ 2 │
/// │   │ │                                                 └───┘
/// │   │ └──  ────  ────  ────  ────  ────  ────  ────  ────  ───┘
/// │   │
/// │   │   ┌────────────────────────┐
/// └───┴──▶│     Update Parent      │  (if inlined on the parent and non-create)
///         └────────────────────────┘
/// ```
/// - Checks in [1] are required because the child exists, which in turn implies that a parent must exist if the relation is required.
///   If this would disconnect the existing parent, we error out. If it doesn't require the parent but exists, we disconnect the relation first.
/// - Checks in [2] are required if the parent is NOT a create operation, as this means the parent record exists in some form. If this disconnects
///   a child record that requires a parent record, we error out. If it doesn't require the parent but exists, we disconnect the relation first.
///
/// Important note: We can't inject directly from the if node into the parent if the parent is a non-create, because we need to perform a check in between,
/// and updating the record with the injection beforehand prevents that check. Instead, we need an additional update.
fn one_to_one_inlined_parent(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
    create_data: ParsedInputMap,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let parent_link = parent_relation_field.linking_fields();
    let child_link = parent_relation_field.related_field().linking_fields();

    let read_node = graph.create_node(utils::read_ids_infallible(
        child_model.clone(),
        child_link.clone(),
        filter.clone(),
    ));

    graph.mark_nodes(&parent_node, &read_node);
    graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;

    let if_node = graph.create_node(Flow::default_if());
    let create_node = create::create_record_node(graph, Arc::clone(child_model), create_data)?;
    let return_existing = graph.create_node(Flow::Return(None));
    let return_create = graph.create_node(Flow::Return(None));

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

    // Then branch handling
    let read_ex_parent_node =
        utils::insert_existing_1to1_related_model_checks(graph, &read_node, &parent_relation_field.related_field())?;

    graph.create_edge(&if_node, &read_ex_parent_node, QueryGraphDependency::Then)?;
    graph.create_edge(
        &read_ex_parent_node,
        &return_existing,
        QueryGraphDependency::ExecutionOrder,
    )?;

    graph.create_edge(
        &read_node,
        &return_existing,
        QueryGraphDependency::ParentProjection(
            child_link.clone(),
            Box::new(move |return_node, child_ids| {
                if let Node::Flow(Flow::Return(_)) = return_node {
                    Ok(Node::Flow(Flow::Return(Some(child_ids))))
                } else {
                    Ok(return_node)
                }
            }),
        ),
    )?;

    // Else branch handling
    graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;
    graph.create_edge(
        &create_node,
        &return_create,
        QueryGraphDependency::ParentProjection(
            child_link.clone(),
            Box::new(move |return_node, child_ids| {
                if let Node::Flow(Flow::Return(_)) = return_node {
                    Ok(Node::Flow(Flow::Return(Some(child_ids))))
                } else {
                    Ok(return_node)
                }
            }),
        ),
    )?;

    if utils::node_is_create(graph, &parent_node) {
        // No need to perform checks, a child can't exist if the parent is just getting created. Simply inject.
        graph.create_edge(
            &if_node,
            &parent_node,
            QueryGraphDependency::ParentProjection(
                child_link.clone(),
                Box::new(move |mut parent, mut child_ids| {
                    let child_id = child_ids.pop().unwrap();
                    if let Node::Query(Query::Write(ref mut wq)) = parent {
                        wq.inject_projection_into_args(parent_link.assimilate(child_id)?);
                    }

                    Ok(parent)
                }),
            ),
        )?;
    } else {
        // Perform checks that no existing child in a required relation is violated.
        graph.create_edge(&if_node, &parent_node, QueryGraphDependency::ExecutionOrder)?;

        let read_ex_child_node =
            utils::insert_existing_1to1_related_model_checks(graph, &parent_node, &parent_relation_field)?;

        let parent_model = parent_relation_field.model();
        let update_parent_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model.clone());

        graph.create_edge(
            &parent_node,
            &update_parent_node,
            QueryGraphDependency::ParentProjection(parent_model.primary_identifier(), Box::new(move |mut update_parent_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(id) => Ok(id),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect or create on a one-to-one relation, updating inlined on parent."))),
                }?;

                if let Node::Query(ref mut q) = update_parent_node {
                    q.add_filter(parent_id.filter());
                }

                Ok(update_parent_node)
            })),
        )?;

        graph.create_edge(
            &if_node,
            &update_parent_node,
            QueryGraphDependency::ParentProjection(child_link, Box::new(move |mut update_parent_node, mut child_projections| {
                let child_projection = match child_projections.pop() {
                    Some(p) => Ok(p),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid child ID to be present for a nested connect or create on a one-to-one relation, updating inlined on parent."))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_parent_node {
                    wq.inject_projection_into_args(parent_link.assimilate(child_projection)?);
                }

                Ok(update_parent_node)
            })),
        )?;
    }

    Ok(())
}

/// Handles one-to-one relations where the inlining is done on the child record
/// The resulting graph:
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// ┌──          Parent         │─────────────────────────┬──────────────────────────────────────────┐
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─                          │                                          │
/// │               │                                     │(If non-create)                           │
/// │                                      ┌───  ────  ───┼  ────  ────  ────  ────  ────  ────  ──┐ │
/// │               │                                     ▼                                        │ │
/// │               ▼                        ┌────────────────────────┐                              │
/// │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─           │ │     Read ex. child     │──┐                           │
/// │         Read Result       │          │ └────────────────────────┘  │                         │ │
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─           │              │              │                         │ │
/// │                                      │              ▼              │(Fail on c > 0 if child  │ │
/// ├───────────────┐                        ┌────────────────────────┐  │     side required)      │ │
/// │               │                        │ If c > 0 && c. inlined │  │                           │
/// │               │                      │ └────────────────────────┘  │                           │
/// │               │                      │         then │              │                         │ │
/// │               │                      │              ▼              │                         │ │
/// │               │                      │ ┌────────────────────────┐  │                         │ │
/// │               │                        │    Update ex. child    │◀─┘                   ┌───┐ │ │
/// │               │                        └────────────────────────┘                      │ 2 │   │
/// │               ▼                      │                                                 └───┘   │
/// │  ┌────────────────────────┐          └──  ────  ────  ────  ────  ────  ────  ────  ────  ───┘ │
/// │  │       Read Child       │━━━┳─────────────────┐                                              │
/// │  └────────────────────────┘   ┃                 │                                              │
/// │               │               ┃                 │                                              │
/// │               │               ┃                 │                                              │
/// │               ▼               ┃                 │                                              │
/// │  ┌────────────────────────┐   ┃                 │                                              │
/// │  │      If (exists)       │───╋──────Then───────┤                                              │
/// │  └────────────────────────┘   ┃                 │                                              │
/// │               │Else           ┃                 │                                              │
/// │               │               ┃                 │                                              │
/// │               ▼               ┃  ┌─── ──── ──── ▼─── ──── ──── ──── ──── ──── ──── ──── ─┐     │
/// │  ┌────────────────────────┐   ┃  │ ┌────────────────────────┐                            │     │
/// └─▶│      Create Child      │   ┃ ┌┼─│    Read ex. Parent     │──┐                         │     │
///    └────────────────────────┘   ┃ ││ └────────────────────────┘  │                               │
///                                 ┃ ││              │              │                         │     │
///                                 ┃ │               ▼              │(Fail on p > 0 if parent │     │
///                                 ┃ ││ ┌────────────────────────┐  │     side required)      │     │
///                                 ┃ ││ │ If p > 0 && p. inlined │  │                         │     │
///                                 ┃ ││ └────────────────────────┘  │                               │
///                                 ┃ ││              │              │                         │     │
///                                 ┃ │               ▼              │                         │     │
///                                 ┃ ││ ┌────────────────────────┐  │                         │     │
///                                 ┃ ││ │   Update ex. parent    │◀─┘                         │     │
///                                 ┃ ││ └────────────────────────┘                      ┌───┐       │
///                                 ┃ ││         then                                    │ 1 │ │     │
///                                 ┃ │                                                  └───┘ │     │
///                                 ┃ │└─── ──── ──── ──── ──── ──── ──── ──── ──── ──── ──── ─┘     │
///                                 ┃ │                                                              │
///                                 ┃ │                                                              │
///                                 ┃ │  ┌────────────────────────┐                                  │
///                                 ┗━┻━▶│      Update Child      │◀─────────────────────────────────┘
///                                      └────────────────────────┘
/// ```
/// - Checks in [1] are required because the child exists, which in turn implies that a parent must exist if the relation is required.
///   If this would disconnect the existing parent, we error out. If it doesn't require the parent but exists, we disconnect the relation first.
/// - Checks in [2] are required if the parent is NOT a create operation, as this means the parent record exists in some form. If this disconnects
///   a child record that requires a parent record, we error out. If it doesn't require the parent but exists, we disconnect the relation first.
///
/// Important note: We can't inject directly from the if node into the parent if the parent is a non-create, because we need to perform a check in between,
/// and updating the record with the injection beforehand prevents that check. Instead, we need an additional update.
fn one_to_one_inlined_child(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
    create_data: ParsedInputMap,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let parent_link = parent_relation_field.linking_fields();
    let child_link = parent_relation_field.related_field().linking_fields();

    let read_node = graph.create_node(utils::read_ids_infallible(
        child_model.clone(),
        child_link.clone(),
        filter.clone(),
    ));

    if !utils::node_is_create(graph, &parent_node) {
        // Perform checks that no existing child in a required relation is violated.
        utils::insert_existing_1to1_related_model_checks(graph, &parent_node, &parent_relation_field)?;
    }

    graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;

    let if_node = graph.create_node(Flow::default_if());
    let create_node = create::create_record_node(graph, Arc::clone(child_model), create_data)?;

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

    // Then branch handling
    let child_model = parent_relation_field.related_model();
    let update_child_node = utils::update_records_node_placeholder(graph, Filter::empty(), child_model.clone());

    let read_ex_parent_node =
        utils::insert_existing_1to1_related_model_checks(graph, &read_node, &parent_relation_field.related_field())?;

    graph.create_edge(&if_node, &read_ex_parent_node, QueryGraphDependency::Then)?;
    graph.create_edge(
        &read_ex_parent_node,
        &update_child_node,
        QueryGraphDependency::ExecutionOrder,
    )?;

    graph.create_edge(
        &read_node,
        &update_child_node,
        QueryGraphDependency::ParentProjection(
            child_model.primary_identifier(),
            Box::new(move |mut update_child_node, mut child_ids| {
                let child_id = match child_ids.pop() {
                    Some(id) => Ok(id),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid child id to be present for a nested connect or create on a one-to-one relation, updating inlined on child."))),
                }?;

                if let Node::Query(ref mut q) = update_child_node {
                    q.add_filter(child_id.filter());
                }

                Ok(update_child_node)
            }),
        ),
    )?;

    graph.create_edge(
        &parent_node,
        &update_child_node,
        QueryGraphDependency::ParentProjection(parent_link.clone(), Box::new(move |mut update_child_node, mut parent_links| {
            let parent_link = match parent_links.pop() {
                Some(link) => Ok(link),
                None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent link to be present for a nested connect or create on a one-to-one relation, updating inlined on child."))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = update_child_node {
                wq.inject_projection_into_args(child_link.assimilate(parent_link)?);
            }

            Ok(update_child_node)
        })),
    )?;

    // Else branch handling
    let child_link = parent_relation_field.related_field().linking_fields();

    graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;
    graph.create_edge(
        &parent_node,
        &create_node,
        QueryGraphDependency::ParentProjection(parent_link, Box::new(move |mut update_child_node, mut parent_links| {
            let parent_link = match parent_links.pop() {
                Some(link) => Ok(link),
                None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent link to be present for a nested connect or create on a one-to-one relation, updating inlined on child."))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = update_child_node {
                wq.inject_projection_into_args(child_link.assimilate(parent_link)?);
            }

            Ok(update_child_node)
        })),
    )?;

    Ok(())
}
