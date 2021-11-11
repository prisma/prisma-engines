use super::*;
use crate::{query_ast::*, query_graph::*, ParsedInputValue};
use connector::Filter;
use itertools::Itertools;
use prisma_models::{ModelRef, RelationFieldRef};
use std::convert::TryInto;
use std::sync::Arc;

/// Only for x-to-many relations.
///
/// Handles nested set cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, value, child_model))]
pub fn nested_set(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    // Build all filters upfront.
    let filters: Vec<Filter> = utils::coerce_vec(value)
        .into_iter()
        .map(|value: ParsedInputValue| {
            let value: ParsedInputMap = value.try_into()?;
            extract_unique_filter(value, &child_model)
        })
        .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?
        .into_iter()
        .unique()
        .collect();

    let filter = Filter::or(filters);

    if relation.is_many_to_many() {
        handle_many_to_many(graph, parent_node, parent_relation_field, filter)
    } else if relation.is_one_to_many() {
        handle_one_to_many(graph, parent_node, parent_relation_field, filter)
    } else {
        panic!("Set is not supported on one-to-one relations.");
    }
}

/// Handles a set on a many-to-many relation.
///
/// The resulting graph:
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent       ──┬ ─ ─ ─ ─ ┐
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘  │
/// │           │           │         │
/// │           │           │
/// │           │           │         │
/// │           ▼           │         ▼
/// │  ┌─────────────────┐  │  ┌ ─ ─ ─ ─ ─ ─ ┐
/// │  │Read old children│  │      Result
/// │  └─────────────────┘  │  └ ─ ─ ─ ─ ─ ─ ┘
/// │           │           │
/// │           │           │
/// │           │           │
/// │           ▼           │
/// │  ┌─────────────────┐  │
/// │  │   Disconnect    │◀─┘
/// │  └─────────────────┘
/// │           │
/// │           │
/// │           │
/// │           ▼
/// │  ┌─────────────────┐
/// │  │Read new children│
/// │  └─────────────────┘
/// │           │
/// │           │
/// │           │
/// │           ▼
/// │  ┌─────────────────┐
/// └─▶│     Connect     │
///    └─────────────────┘
/// ```
///
/// Connects only happen if the query specifies at least one record to be connected.
/// If none are specified, set effectively acts as a "disconnect all".
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, filter))]
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
) -> QueryGraphBuilderResult<()> {
    let parent_model_identifier = parent_relation_field.model().primary_identifier();
    let child_model = parent_relation_field.related_model();
    let child_model_identifier = child_model.primary_identifier();
    let read_old_node =
        utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, Filter::empty())?;

    let disconnect = WriteQuery::DisconnectRecords(DisconnectRecords {
        parent_id: None,
        child_ids: vec![],
        relation_field: Arc::clone(parent_relation_field),
    });

    let disconnect_node = graph.create_node(Query::Write(disconnect));
    let relation_name = parent_relation_field.relation().name.clone();
    let parent_model_name = parent_relation_field.model().name.clone();

    // Edge from parent to disconnect
    graph.create_edge(
         parent_node,
         &disconnect_node,
         QueryGraphDependency::ParentProjection(parent_model_identifier, Box::new(move |mut disconnect_node, mut parent_ids| {
             let parent_id = match parent_ids.pop() {
                 Some(pid) => Ok(pid),
                 None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "No '{}' records (needed to disconnect existing child records) were found for a nested set on many-to-many relation '{}'.",
                    parent_model_name, relation_name
                ))),
             }?;

             if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = disconnect_node {
                 c.parent_id = Some(parent_id);
             }

             Ok(disconnect_node)
         })),
     )?;

    // Edge from read to disconnect.
    graph.create_edge(
        &read_old_node,
        &disconnect_node,
        QueryGraphDependency::ParentProjection(
            child_model_identifier.clone(),
            Box::new(|mut disconnect_node, child_ids| {
                // todo: What if there are no connected nodes to disconnect?
                if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = disconnect_node {
                    c.child_ids = child_ids;
                }

                Ok(disconnect_node)
            }),
        ),
    )?;

    if filter.size() > 0 {
        let expected_connects = filter.size();
        let read_new_query = utils::read_ids_infallible(child_model, child_model_identifier, filter);
        let read_new_node = graph.create_node(read_new_query);

        graph.create_edge(&disconnect_node, &read_new_node, QueryGraphDependency::ExecutionOrder)?;

        connect::connect_records_node(
            graph,
            parent_node,
            &read_new_node,
            parent_relation_field,
            expected_connects,
        )?;
    }

    Ok(())
}

/// Handles a nested many-to-one set scenario.
/// Set only works on lists.
/// This implies that `parent` can only ever be the to-one side, and child can only be the many (inlined) side.
///
/// ```text
///      ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌────      Parent       ─ ─ ─ ─ ─ ─ ┐
/// │    └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │             │                     │
/// │             │
/// │             │                     │
/// │             ▼                     ▼
/// │    ┌─────────────────┐     ┌ ─ ─ ─ ─ ─ ─ ┐
/// │ ┌──│Read old children│         Result
/// │ │  └─────────────────┘     └ ─ ─ ─ ─ ─ ─ ┘
/// │ │           │
/// │ │           │
/// │ │           │
/// │ │           ▼
/// │ │  ┌─────────────────┐
/// │ │  │Read new children│
/// │ │  └─────────────────┘
/// │ │           │
/// │ │           │
/// │ │           │
/// │ │           ▼
/// │ │  ┌─────────────────┐
/// │ └─▶│      Diff       │──────────────────────────┐
/// │    └─────────────────┘                          │
/// │             │                                   │
/// │ ┌───────────┼───────────────────────┐           │
/// │ │           │                       │           │
/// │ │           ▼                       ▼           │
/// │ │  ┌─────────────────┐     ┌─────────────────┐  │
/// │ │  │  If (left > 0)  │     │ If (right > 0)  │  │
/// │ │  └─────────────────┘     └─────────────────┘  │
/// │ │           │                       │           │
/// │ │           │                       │           │
/// │ │           │                       │           │
/// │ │           ▼                       ▼           │
/// │ │  ┌─────────────────┐     ┌─────────────────┐  │
/// │ │  │ Update children │     │ Update children │  │
/// └─┴─▶│   ("connect")   │     │ ("disconnect")  │◀─┘
///      └─────────────────┘     └─────────────────┘
/// ```
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, filter))]
fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
) -> QueryGraphBuilderResult<()> {
    let child_model_identifier = parent_relation_field.related_model().primary_identifier();
    let child_link = parent_relation_field.related_field().linking_fields();
    let parent_link = parent_relation_field.linking_fields();
    let empty_child_link = child_link.empty_record_projection();

    let child_model = parent_relation_field.related_model();
    let read_old_node =
        utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, Filter::empty())?;

    let read_new_query = utils::read_ids_infallible(child_model.clone(), child_model_identifier.clone(), filter);
    let read_new_node = graph.create_node(read_new_query);
    let diff_node = graph.create_node(Node::Computation(Computation::empty_diff()));

    graph.create_edge(&read_old_node, &read_new_node, QueryGraphDependency::ExecutionOrder)?;

    // The new IDs that are not yet connected will be on the `left` side of the diff.
    graph.create_edge(
        &read_new_node,
        &diff_node,
        QueryGraphDependency::ParentProjection(
            child_model_identifier.clone(),
            Box::new(move |mut diff_node, child_ids| {
                if let Node::Computation(Computation::Diff(ref mut diff)) = diff_node {
                    diff.left = child_ids.into_iter().collect();
                }

                Ok(diff_node)
            }),
        ),
    )?;

    // The old IDs that must be disconnected will be on the `right` side of the diff.
    graph.create_edge(
        &read_old_node,
        &diff_node,
        QueryGraphDependency::ParentProjection(
            child_model_identifier,
            Box::new(move |mut diff_node, child_ids| {
                if let Node::Computation(Computation::Diff(ref mut diff)) = diff_node {
                    diff.right = child_ids.into_iter().collect();
                }

                Ok(diff_node)
            }),
        ),
    )?;

    // Update (connect) case: Check left diff IDs
    let connect_if_node = graph.create_node(Node::Flow(Flow::default_if()));
    let update_connect_node = utils::update_records_node_placeholder(graph, Filter::empty(), Arc::clone(&child_model));

    graph.create_edge(
        &diff_node,
        &connect_if_node,
        QueryGraphDependency::ParentResult(Box::new(move |connect_if_node, result| {
            let diff_result = result.as_diff_result().unwrap();
            let should_connect = !diff_result.left.is_empty();

            if let Node::Flow(Flow::If(_)) = connect_if_node {
                Ok(Node::Flow(Flow::If(Box::new(move || should_connect))))
            } else {
                unreachable!()
            }
        })),
    )?;

    let relation_name = parent_relation_field.relation().name.clone();
    let parent_model_name = parent_relation_field.model().name.clone();

    // Connect to the if node, the parent node (for the inlining ID) and the diff node (to get the IDs to update)
    graph.create_edge(&connect_if_node, &update_connect_node, QueryGraphDependency::Then)?;
    graph.create_edge(
        parent_node,
        &update_connect_node,
        QueryGraphDependency::ParentProjection(
            parent_link,
            Box::new(move |mut update_connect_node, mut parent_links| {
                let parent_link = match parent_links.pop() {
                    Some(link) => Ok(link),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{}' records were found for a nested set on many-to-many relation '{}'.",
                        parent_model_name, relation_name
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_connect_node {
                    wq.inject_projection_into_args(child_link.assimilate(parent_link)?);
                }

                Ok(update_connect_node)
            }),
        ),
    )?;

    graph.create_edge(
        &diff_node,
        &update_connect_node,
        QueryGraphDependency::ParentResult(Box::new(move |mut update_connect_node, result| {
            let diff_result = result.as_diff_result().unwrap();

            if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = update_connect_node {
                ur.record_filter = diff_result.left.clone().into();
            }

            Ok(update_connect_node)
        })),
    )?;

    // Update (disconnect) case: Check right diff IDs.
    let disconnect_if_node = graph.create_node(Node::Flow(Flow::default_if()));
    let update_disconnect_node =
        utils::update_records_node_placeholder(graph, Filter::empty(), Arc::clone(&child_model));

    let child_side_required = parent_relation_field.related_field().is_required();
    let rf = Arc::clone(parent_relation_field);

    graph.create_edge(
        &diff_node,
        &disconnect_if_node,
        QueryGraphDependency::ParentResult(Box::new(move |node, result| {
            let diff_result = result.as_diff_result().unwrap();
            let should_connect = !diff_result.right.is_empty();

            if should_connect && child_side_required {
                return Err(QueryGraphBuilderError::RelationViolation(rf.into()));
            }

            if let Node::Flow(Flow::If(_)) = node {
                Ok(Node::Flow(Flow::If(Box::new(move || should_connect))))
            } else {
                unreachable!()
            }
        })),
    )?;

    // Connect to the if node and the diff node (to get the IDs to update)
    graph.create_edge(&disconnect_if_node, &update_disconnect_node, QueryGraphDependency::Then)?;
    graph.create_edge(
        &diff_node,
        &update_disconnect_node,
        QueryGraphDependency::ParentResult(Box::new(move |mut node, result| {
            let diff_result = result.as_diff_result().unwrap();

            if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = node {
                ur.record_filter = diff_result.right.clone().into();
            }

            if let Node::Query(Query::Write(ref mut wq)) = node {
                wq.inject_projection_into_args(empty_child_link);
            }

            Ok(node)
        })),
    )?;

    Ok(())
}
