use super::*;
use crate::{query_ast::*, query_graph::*, ParsedInputValue};
use itertools::Itertools;
use prisma_models::{GraphqlId, ModelRef, PrismaValue, RelationFieldRef};
use std::{collections::HashSet, convert::TryInto, iter::FromIterator, sync::Arc};

/// Only for x-to-many relations.
///
/// Handles nested set cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn connect_nested_set(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    // Build all finders upfront.
    let finders: Vec<RecordFinder> = utils::coerce_vec(value)
        .into_iter()
        .map(|value: ParsedInputValue| extract_record_finder(value, &child_model))
        .collect::<QueryGraphBuilderResult<Vec<RecordFinder>>>()?
        .into_iter()
        .unique()
        .collect();

    if relation.is_many_to_many() {
        handle_many_to_many(graph, parent_node, parent_relation_field, finders)
    } else if relation.is_one_to_many() {
        handle_one_to_many(graph, parent_node, parent_relation_field, finders)
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
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    finders: Vec<RecordFinder>,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();
    let read_old_node = utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, None)?;

    let disconnect = WriteQuery::DisconnectRecords(DisconnectRecords {
        parent_id: None,
        child_ids: vec![],
        relation_field: Arc::clone(parent_relation_field),
    });

    let disconnect_node = graph.create_node(Query::Write(disconnect));

    // Edge from parent to disconnect
    graph.create_edge(
        parent_node,
        &disconnect_node,
        QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!(
                    "[Query Graph] Expected a valid parent ID to be present for a nested set (disconnect part) on a many-to-many relation."
                ))),
            }?;

            if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = child_node {
                c.parent_id = Some(parent_id.try_into()?);
            }

            Ok(child_node)
        })),
    )?;

    // Edge from read to disconnect.
    graph.create_edge(
        &read_old_node,
        &disconnect_node,
        QueryGraphDependency::ParentIds(Box::new(|mut disconnect_node, parent_ids| {
            // todo: What if there are no connected nodes to disconnect?
            if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = disconnect_node {
                c.child_ids = parent_ids.into_iter().map(|id| id.try_into().unwrap()).collect();
            }

            Ok(disconnect_node)
        })),
    )?;

    if finders.len() > 0 {
        let expected_connects = finders.len();
        let read_new_query = utils::read_ids_infallible(&child_model, finders);
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
fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    finders: Vec<RecordFinder>,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();
    let read_old_node = utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, None)?;
    let read_new_query = utils::read_ids_infallible(&child_model, finders);
    let read_new_node = graph.create_node(read_new_query);
    let diff_node = graph.create_node(Node::Computation(Computation::empty_diff()));

    graph.create_edge(&read_old_node, &read_new_node, QueryGraphDependency::ExecutionOrder)?;

    // The new IDs that are not yet connected will be on the `left` side of the diff.
    graph.create_edge(
        &read_new_node,
        &diff_node,
        QueryGraphDependency::ParentIds(Box::new(move |mut node, parent_ids| {
            if let Node::Computation(Computation::Diff(ref mut diff)) = node {
                let parent_ids: Vec<GraphqlId> = parent_ids.into_iter().map(|i| i.try_into().unwrap()).collect();
                diff.left = HashSet::from_iter(parent_ids.into_iter());
            }

            Ok(node)
        })),
    )?;

    // The old IDs that must be disconnected will be on the `right` side of the diff.
    graph.create_edge(
        &read_old_node,
        &diff_node,
        QueryGraphDependency::ParentIds(Box::new(move |mut node, parent_ids| {
            if let Node::Computation(Computation::Diff(ref mut diff)) = node {
                let parent_ids: Vec<GraphqlId> = parent_ids.into_iter().map(|i| i.try_into().unwrap()).collect();
                diff.right = HashSet::from_iter(parent_ids.into_iter());
            }

            Ok(node)
        })),
    )?;

    // Update (connect) case: Check left diff IDs
    let connect_if_node = graph.create_node(Node::Flow(Flow::default_if()));
    let update_connect_node = utils::update_records_node_placeholder(graph, None, Arc::clone(&child_model));
    let relation_field_name = parent_relation_field.related_field().name.clone();

    graph.create_edge(
        &diff_node,
        &connect_if_node,
        QueryGraphDependency::ParentResult(Box::new(move |node, result| {
            let diff_result = result.as_diff_result().unwrap();
            let should_connect = !diff_result.left.is_empty();

            if let Node::Flow(Flow::If(_)) = node {
                Ok(Node::Flow(Flow::If(Box::new(move || should_connect))))
            } else {
                unreachable!()
            }
        })),
    )?;

    // Connect to the if node, the parent node (for the inlining ID) and the diff node (to get the IDs to update)
    graph.create_edge(&connect_if_node, &update_connect_node, QueryGraphDependency::Then)?;
    graph.create_edge(
        parent_node,
        &update_connect_node,
        QueryGraphDependency::ParentIds(Box::new(move |mut node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!(
                "[Query Graph] Expected a valid parent ID to be present for a nested set on a one-to-many relation."
            ))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = node {
                wq.inject_non_list_arg(relation_field_name, parent_id);
            }

            Ok(node)
        })),
    )?;

    let id_field = child_model.fields().id();
    graph.create_edge(
        &diff_node,
        &update_connect_node,
        QueryGraphDependency::ParentResult(Box::new(move |mut node, result| {
            let diff_result = result.as_diff_result().unwrap();

            if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = node {
                ur.filter = diff_result
                    .left
                    .iter()
                    .map(|id| RecordFinder {
                        field: id_field.clone(),
                        value: id.into(),
                    })
                    .collect::<Vec<RecordFinder>>()
                    .into();
            }

            Ok(node)
        })),
    )?;

    // Update (disconnct) case: Check right diff IDs.
    let disconnect_if_node = graph.create_node(Node::Flow(Flow::default_if()));
    let update_disconnect_node = utils::update_records_node_placeholder(graph, None, Arc::clone(&child_model));
    let relation_field_name = parent_relation_field.related_field().name.clone();
    let child_side_required = parent_relation_field.related_field().is_required;
    let rf = Arc::clone(parent_relation_field);
    let id_field = child_model.fields().id();

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
                ur.filter = diff_result
                    .right
                    .iter()
                    .map(|id| RecordFinder {
                        field: id_field.clone(),
                        value: id.into(),
                    })
                    .collect::<Vec<RecordFinder>>()
                    .into();
            }

            if let Node::Query(Query::Write(ref mut wq)) = node {
                wq.inject_non_list_arg(relation_field_name, PrismaValue::Null);
            }

            Ok(node)
        })),
    )?;

    Ok(())
}
