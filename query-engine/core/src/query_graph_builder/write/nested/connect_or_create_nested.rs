use super::*;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    Computation, ParsedInputMap, ParsedInputValue,
};
use query_structure::{Filter, IntoFilter, Model, RelationFieldRef, SelectionResult};
use schema::constants::args;
use std::convert::TryInto;

/// Handles nested connect or create cases.
///
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub(crate) fn nested_connect_or_create(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();
    let values = utils::coerce_vec(value);

    if relation.is_many_to_many() {
        handle_many_to_many(
            graph,
            query_schema,
            parent_node,
            parent_relation_field,
            values,
            child_model,
        )
    } else if relation.is_one_to_many() {
        handle_one_to_many(
            graph,
            query_schema,
            parent_node,
            parent_relation_field,
            values,
            child_model,
        )
    } else {
        handle_one_to_one(
            graph,
            query_schema,
            parent_node,
            parent_relation_field,
            values,
            child_model,
        )
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
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue<'_>>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    for value in values {
        let mut value: ParsedInputMap<'_> = value.try_into()?;

        let where_arg = value.swap_remove(args::WHERE).unwrap();
        let where_map: ParsedInputMap<'_> = where_arg.try_into()?;

        let create_arg = value.swap_remove(args::CREATE).unwrap();
        let create_map: ParsedInputMap<'_> = create_arg.try_into()?;

        let filter = extract_unique_filter(where_map, child_model)?;
        let read_node = graph.create_node(utils::read_ids_infallible(
            child_model.clone(),
            child_model.primary_identifier(),
            filter,
        ));

        let create_node = create::create_record_node(graph, query_schema, child_model.clone(), create_map)?;
        let if_node = graph.create_node(Flow::default_if());

        let connect_exists_node =
            connect::connect_records_node(graph, &parent_node, &read_node, parent_relation_field, 1)?;

        let _connect_create_node =
            connect::connect_records_node(graph, &parent_node, &create_node, parent_relation_field, 1)?;

        graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;
        graph.create_edge(
            &read_node,
            &if_node,
            QueryGraphDependency::ProjectedDataDependency(
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
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue<'_>>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    if parent_relation_field.is_inlined_on_enclosing_model() {
        one_to_many_inlined_parent(
            graph,
            query_schema,
            parent_node,
            parent_relation_field,
            values,
            child_model,
        )
    } else {
        one_to_many_inlined_child(
            graph,
            query_schema,
            parent_node,
            parent_relation_field,
            values,
            child_model,
        )
    }
}

/// Dispatcher for one-to-one relations.
fn handle_one_to_one(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut values: Vec<ParsedInputValue<'_>>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let value = values.pop().unwrap();
    let mut value: ParsedInputMap<'_> = value.try_into()?;

    let where_arg = value.swap_remove(args::WHERE).unwrap();
    let where_map: ParsedInputMap<'_> = where_arg.try_into()?;

    let create_arg = value.swap_remove(args::CREATE).unwrap();
    let create_data: ParsedInputMap<'_> = create_arg.try_into()?;

    let filter = extract_unique_filter(where_map, child_model)?;

    if parent_relation_field.is_inlined_on_enclosing_model() {
        one_to_one_inlined_parent(
            graph,
            query_schema,
            parent_node,
            parent_relation_field,
            filter,
            create_data,
            child_model,
        )
    } else {
        one_to_one_inlined_child(
            graph,
            query_schema,
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
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue<'_>>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    for value in values {
        let parent_link = parent_relation_field.linking_fields();
        let child_link = parent_relation_field.related_field().linking_fields();

        let mut value: ParsedInputMap<'_> = value.try_into()?;

        let where_arg = value.swap_remove(args::WHERE).unwrap();
        let where_map: ParsedInputMap<'_> = where_arg.try_into()?;

        let create_arg = value.swap_remove(args::CREATE).unwrap();
        let create_map: ParsedInputMap<'_> = create_arg.try_into()?;

        let filter = extract_unique_filter(where_map, child_model)?;
        let read_node = graph.create_node(utils::read_ids_infallible(
            child_model.clone(),
            child_link.clone(),
            filter.clone(),
        ));

        let if_node = graph.create_node(Flow::default_if());
        let update_child_node = utils::update_records_node_placeholder(graph, filter, child_model.clone());
        let create_node = create::create_record_node(graph, query_schema, child_model.clone(), create_map)?;

        graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;
        graph.create_edge(&if_node, &update_child_node, QueryGraphDependency::Then)?;
        graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;
        graph.create_edge(
            &read_node,
            &if_node,
            QueryGraphDependency::ProjectedDataDependency(
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

        let relation_name = parent_relation_field.relation().name().to_owned();
        let parent_model_name = parent_relation_field.model().name().to_owned();
        let child_model_name = child_model.name().to_owned();

        graph.create_edge(
            &parent_node,
            &create_node,
            QueryGraphDependency::ProjectedDataDependency(
                parent_link.clone(),
                Box::new(move |mut create_node, mut parent_ids| {
                    let parent_id = match parent_ids.pop() {
                        Some(id) => Ok(id),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{child_model_name}' record (needed to inline the relation with a create on '{parent_model_name}' record(s)) was found for a nested connect or create on one-to-many relation '{relation_name}'."
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = create_node {
                        wq.inject_result_into_args(child_link.assimilate(parent_id)?);
                    }

                    Ok(create_node)
                }),
            ),
        )?;

        let relation_name = parent_relation_field.relation().name().to_owned();
        let parent_model_name = parent_relation_field.model().name().to_owned();
        let child_model_name = child_model.name().to_owned();
        let child_link = parent_relation_field.related_field().linking_fields();

        graph.create_edge(
            &parent_node,
            &update_child_node,
            QueryGraphDependency::ProjectedDataDependency(
                parent_link,
                Box::new(move |mut update_node, mut parent_ids| {
                    let parent_id = match parent_ids.pop() {
                        Some(id) => Ok(id),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{child_model_name}' record (needed to inline the relation the update for '{parent_model_name}' record(s)) was found for a nested connect or create on one-to-many relation '{relation_name}'."
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_node {
                        wq.inject_result_into_args(child_link.assimilate(parent_id)?);
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
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut values: Vec<ParsedInputValue<'_>>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let parent_link = parent_relation_field.linking_fields();
    let child_link = parent_relation_field.related_field().linking_fields();

    let value = values.pop().unwrap();
    let mut value: ParsedInputMap<'_> = value.try_into()?;

    let where_arg = value.swap_remove(args::WHERE).unwrap();
    let where_map: ParsedInputMap<'_> = where_arg.try_into()?;

    let create_arg = value.swap_remove(args::CREATE).unwrap();
    let create_map: ParsedInputMap<'_> = create_arg.try_into()?;

    let filter = extract_unique_filter(where_map, child_model)?;
    let read_node = graph.create_node(utils::read_ids_infallible(
        child_model.clone(),
        child_link.clone(),
        filter,
    ));

    graph.mark_nodes(&parent_node, &read_node);
    graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;

    let if_node = graph.create_node(Flow::default_if());
    let create_node = create::create_record_node(graph, query_schema, child_model.clone(), create_map)?;
    let return_existing = graph.create_node(Flow::Return(None));
    let return_create = graph.create_node(Flow::Return(None));

    graph.create_edge(
        &read_node,
        &if_node,
        QueryGraphDependency::ProjectedDataDependency(
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
        QueryGraphDependency::ProjectedDataDependency(
            child_link.clone(),
            Box::new(move |mut parent, mut child_ids| {
                let child_id = child_ids.pop().unwrap();
                if let Node::Query(Query::Write(ref mut wq)) = parent {
                    wq.inject_result_into_args(parent_link.assimilate(child_id)?);
                }

                Ok(parent)
            }),
        ),
    )?;

    graph.create_edge(
        &read_node,
        &return_existing,
        QueryGraphDependency::ProjectedDataDependency(
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
        QueryGraphDependency::ProjectedDataDependency(
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
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
    create_data: ParsedInputMap<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let parent_link = parent_relation_field.linking_fields();
    let child_link = parent_relation_field.related_field().linking_fields();

    let read_node = graph.create_node(utils::read_ids_infallible(
        child_model.clone(),
        child_link.clone(),
        filter,
    ));

    graph.mark_nodes(&parent_node, &read_node);
    graph.create_edge(&parent_node, &read_node, QueryGraphDependency::ExecutionOrder)?;

    let if_node = graph.create_node(Flow::default_if());
    let create_node = create::create_record_node(graph, query_schema, child_model.clone(), create_data)?;
    let return_existing = graph.create_node(Flow::Return(None));
    let return_create = graph.create_node(Flow::Return(None));

    graph.create_edge(
        &read_node,
        &if_node,
        QueryGraphDependency::ProjectedDataDependency(
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
        QueryGraphDependency::ProjectedDataDependency(
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
        QueryGraphDependency::ProjectedDataDependency(
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
            QueryGraphDependency::ProjectedDataDependency(
                child_link,
                Box::new(move |mut parent, mut child_ids| {
                    let child_id = child_ids.pop().unwrap();
                    if let Node::Query(Query::Write(ref mut wq)) = parent {
                        wq.inject_result_into_args(parent_link.assimilate(child_id)?);
                    }

                    Ok(parent)
                }),
            ),
        )?;
    } else {
        // Perform checks that no existing child in a required relation is violated.
        graph.create_edge(&if_node, &parent_node, QueryGraphDependency::ExecutionOrder)?;
        utils::insert_existing_1to1_related_model_checks(graph, &parent_node, parent_relation_field)?;

        let parent_model = parent_relation_field.model();
        let update_parent_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model.clone());
        let relation_name = parent_relation_field.relation().name();
        let parent_model_name = parent_model.name().to_owned();
        let child_model_name = child_model.name().to_owned();

        graph.create_edge(
            &parent_node,
            &update_parent_node,
            QueryGraphDependency::ProjectedDataDependency(parent_model.primary_identifier(), Box::new(move |mut update_parent_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(id) => Ok(id),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{child_model_name}' record (needed to inline the relation with an update on '{parent_model_name}' record(s)) was found for a nested connect or create on one-to-one relation '{relation_name}'."
                    ))),
                }?;

                if let Node::Query(ref mut q) = update_parent_node {
                    q.add_filter(parent_id.filter());
                }

                Ok(update_parent_node)
            })),
        )?;

        let relation_name = parent_relation_field.relation().name();
        let parent_model_name = parent_model.name().to_owned();
        let child_model_name = child_model.name().to_owned();

        graph.create_edge(
            &if_node,
            &update_parent_node,
            QueryGraphDependency::ProjectedDataDependency(child_link, Box::new(move |mut update_parent_node, mut child_results| {
                let child_result = match child_results.pop() {
                    Some(p) => Ok(p),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{child_model_name}' record (needed to inline the relation with an update on '{parent_model_name}' record(s)) was found for a nested connect or create on one-to-one relation '{relation_name}'."
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_parent_node {
                    wq.inject_result_into_args(parent_link.assimilate(child_result)?);
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
///            ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// ┌──────┬───          Parent         │──────────────────────────┬──────────────────────────┐
/// │      │   └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─                           │                          │
/// │      │                │                                      │                          │
/// │      │                │                                      │                          │
/// │      │                ▼                                      │                          │
/// │      │   ┌────────────────────────┐                          │                          │
/// │ ┌──┬─┼───│     Read new child     │────────────────────────────────────────────────┐    │
/// │ │  │ │   └────────────────────────┘                          │                     │    │
/// │ │  │ │                │                                      │                     │    │
/// │ │  │ │                │                                      │                     │    │
/// │ │  │ │                ▼                                      ▼                     │    │
/// │ │  │ │   ┌────────────────────────┐             ┌────────────────────────┐         │    │
/// │ │  │ │   │      If (exists)       │──────Else──▶│      Create Child      │         │    │
/// │ │  │ │   └────────────────────────┘             └────────────────────────┘         │    │
/// │ │  │ │                │                                                            │    │
/// │ │  │ │                │                                                            │    │
/// │ │  │ │              Then                                                           │    │
/// │ │  │ │                │                                 (If create)                │    │
/// │ │  │ │                ├─────────────────────────────────────────────┐              │    │
/// │ │  │ │                │                                             │              │    │
/// │ │  │ │                │(If non-create)                              │              │    │
/// │ │  │ │┌───  ────  ────│ ────  ────  ────  ────  ──┐  ┌───  ────  ───┼  ────  ────  │    │
/// │ │  │ ││               ▼                           │                 ▼              │    │
/// │ │  │ ││  ┌────────────────────────┐                    ┌────────────────────────┐  │    │
/// │ │  │ └┼─▶│     Read old child     │────────┐         │ │    Update new child    │◀─┘────┘
/// │ │  │  │  └────────────────────────┘        │      │  │ └────────────────────────┘
/// │ │  │                  │                    │      │  │
/// │ │  │                  ▼                    │      │  └  ────  ────  ────  ────  ───┘
/// │ │  │  │  ┌────────────────────────┐        │      │
/// │ │  └──┼─▶│          Diff          │        │
/// │ │     │  └────────────────────────┘     Fail if
/// │ │     │               │               relation to │
/// │ │                     ▼                 parent    │
/// │ │        ┌────────────────────────┐    required   │
/// │ │     │  │   If (not the same)    │        │      │
/// │ │     │  └────────────────────────┘        │
/// │ │     │               │                    │
/// │ │     │             Then                   │      │
/// │ │                     │                    │      │
/// │ │                     ▼                    │      │
/// │ │     │  ┌────────────────────────┐        │      │
/// │ │     │  │    Update old child    │        │
/// │ │     │  │      (disconnect)      │◀───────┘
/// │ │     │  └────────────────────────┘               │
/// │ │                     │                           │
/// │ │                     │                           │
/// │ │     │               ▼                           │
/// │ │     │  ┌────────────────────────┐
/// └─┴─────┼─▶│    Update new child    │
///         │  └────────────────────────┘               │
///                                                     │
///          ────  ────  ────  ────  ────  ────  ────  ─┘
/// ```
/// Note that two versions of this graph can be build: the create and non-create case,
/// but they're never build at the same time (denoted by the dashed boxes).
fn one_to_one_inlined_child(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
    create_data: ParsedInputMap<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let child_model_identifier = child_model.primary_identifier();
    let parent_link = parent_relation_field.linking_fields();
    let child_link = parent_relation_field.related_field().linking_fields();
    let child_relation_field = parent_relation_field.related_field();

    let read_new_child_node = graph.create_node(utils::read_ids_infallible(
        child_model.clone(),
        child_link.clone(),
        filter,
    ));

    // Edge: Parent -> read new child
    graph.create_edge(&parent_node, &read_new_child_node, QueryGraphDependency::ExecutionOrder)?;

    let if_node = graph.create_node(Flow::default_if());
    let create_node = create::create_record_node(graph, query_schema, child_model.clone(), create_data)?;

    // Edge: Read new child -> if node
    graph.create_edge(
        &read_new_child_node,
        &if_node,
        QueryGraphDependency::ProjectedDataDependency(
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

    // *** Else branch handling ***
    // Edge: If node -> create node (else)
    graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;

    // *** Then branch handling ***
    let update_new_child_node = utils::update_records_node_placeholder(graph, Filter::empty(), child_model.clone());
    let relation_name = parent_relation_field.relation().name();
    let parent_model_name = parent_relation_field.model().name().to_owned();
    let child_model_name = child_model.name().to_owned();

    // Edge: Parent node -> update new child node
    graph.create_edge(
        &parent_node,
        &update_new_child_node,
        QueryGraphDependency::ProjectedDataDependency(parent_link.clone(), Box::new(move |mut update_new_child_node, mut parent_links| {
            let parent_link = match parent_links.pop() {
                Some(link) => Ok(link),
                None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "No '{parent_model_name}' record (needed to find '{child_model_name}' record(s) to update) was found for a nested connect or create on one-to-one relation '{relation_name}'."
                ))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = update_new_child_node {
                wq.inject_result_into_args(child_link.assimilate(parent_link)?);
            }

            Ok(update_new_child_node)
        })),
    )?;

    let relation_name = parent_relation_field.relation().name();
    let parent_model_name = parent_relation_field.model().name().to_owned();
    let child_model_name = child_model.name().to_owned();
    let child_link = parent_relation_field.related_field().linking_fields();

    // Edge: Parent node -> create new child node
    graph.create_edge(
        &parent_node,
        &create_node,
        QueryGraphDependency::ProjectedDataDependency(parent_link, Box::new(move |mut create_node, mut parent_links| {
            let parent_link = match parent_links.pop() {
                Some(link) => Ok(link),
                None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "No '{parent_model_name}' record (needed to inline relation with create on '{child_model_name}' record(s)) was found for a nested connect or create on one-to-one relation '{relation_name}'."
                ))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = create_node {
                wq.inject_result_into_args(child_link.assimilate(parent_link)?);
            }

            Ok(create_node)
        })),
    )?;

    let relation_name = parent_relation_field.relation().name();
    let parent_model_name = parent_relation_field.model().name().to_owned();
    let child_model_name = child_model.name().to_owned();
    let child_link = parent_relation_field.related_field().linking_fields();

    // Edge: Read new child node -> update new child node
    graph.create_edge(
        &read_new_child_node,
        &update_new_child_node,
        QueryGraphDependency::ProjectedDataDependency(child_model_identifier.clone(), Box::new(move |mut update_new_child_node, mut new_child_ids| {
            let old_child_id = match new_child_ids.pop() {
                Some(id) => Ok(id),
                None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "No '{parent_model_name}' record (needed to find '{child_model_name}' record(s) to update) was found for a nested connect or create on one-to-one relation '{relation_name}'."
                ))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = update_new_child_node {
                wq.add_filter(old_child_id.filter());
            }

            Ok(update_new_child_node)
        })),
    )?;

    if utils::node_is_create(graph, &parent_node) {
        // 1) A create can't have a previous child connected, we can skip those checks.
        // 2) Since the relation is inlined in the child, we can simply override the old value, it will automatically disconnect the old one.
        // 3) The parent -> old child relationship can't be required, so it's always okay to disconnect.

        // Edge: If node -> update new child node
        graph.create_edge(&if_node, &update_new_child_node, QueryGraphDependency::Then)?;
    } else {
        let read_old_child_node =
            utils::insert_find_children_by_parent_node(graph, &parent_node, parent_relation_field, Filter::empty())?;

        // Edge: If node -> read old child node
        graph.create_edge(&if_node, &read_old_child_node, QueryGraphDependency::Then)?;

        let diff_node = graph.create_node(Node::Computation(Computation::empty_diff()));

        // Edge: Read old child node -> diff node
        graph.create_edge(
            &read_new_child_node,
            &diff_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier.clone(),
                Box::new(move |mut diff_node, child_ids| {
                    if let Node::Computation(Computation::Diff(ref mut diff)) = diff_node {
                        diff.left = child_ids.into_iter().collect();
                    }

                    Ok(diff_node)
                }),
            ),
        )?;

        // Edge: Read old child node -> diff node
        graph.create_edge(
            &read_old_child_node,
            &diff_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier.clone(),
                Box::new(move |mut diff_node, child_ids| {
                    if let Node::Computation(Computation::Diff(ref mut diff)) = diff_node {
                        diff.right = child_ids.into_iter().collect();
                    }

                    Ok(diff_node)
                }),
            ),
        )?;

        let if_node = graph.create_node(Flow::default_if());
        graph.create_edge(
            &diff_node,
            &if_node,
            QueryGraphDependency::DataDependency(Box::new(move |if_node, result| {
                let diff_result = result.as_diff_result().unwrap();
                let should_disconnect = !diff_result.left.is_empty();

                if let Node::Flow(Flow::If(_)) = if_node {
                    Ok(Node::Flow(Flow::If(Box::new(move || should_disconnect))))
                } else {
                    unreachable!()
                }
            })),
        )?;

        // update old child, set link to null
        let update_old_child_node = utils::update_records_node_placeholder(graph, Filter::empty(), child_model.clone());
        let relation_name = parent_relation_field.relation().name();
        let parent_model_name = parent_relation_field.model().name().to_owned();
        let child_model_name = child_model.name().to_owned();
        let rf = parent_relation_field.clone();

        // Edge: Read old child node -> update old child
        graph.create_edge(
            &read_old_child_node,
            &update_old_child_node,
            QueryGraphDependency::ProjectedDataDependency(child_model_identifier, Box::new(move |mut update_old_child_node, mut old_child_ids| {
                if child_relation_field.is_required() && !old_child_ids.is_empty() {
                    return Err(QueryGraphBuilderError::RelationViolation(rf.into()));
                }

                // If there's no child connected, don't attempt to disconnect it.
                if old_child_ids.is_empty() {
                    return Ok(update_old_child_node);
                }

                let old_child_id = match old_child_ids.pop() {
                    Some(id) => Ok(id),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{parent_model_name}' record (needed to find '{child_model_name}' record(s) to update) was found for a nested connect or create on one-to-one relation '{relation_name}'."
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_old_child_node {
                    wq.add_filter(old_child_id.filter());
                    wq.inject_result_into_args(SelectionResult::from(&child_link));
                }

                Ok(update_old_child_node)
            })),
        )?;

        graph.create_edge(&if_node, &update_old_child_node, QueryGraphDependency::Then)?;
        graph.create_edge(
            &update_old_child_node,
            &update_new_child_node,
            QueryGraphDependency::ExecutionOrder,
        )?;
    }

    Ok(())
}
