use super::*;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use itertools::Itertools;
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

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

fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    finders: Vec<RecordFinder>,
) -> QueryGraphBuilderResult<()> {
    unimplemented!();
}

// let mut finders = Vec::new();
// for value in utils::coerce_vec(value) {
//     let record_finder = extract_record_finder(value, &child_model)?;
//     finders.push(record_finder);
// }

// let child_read_query = utils::read_ids_infallible(&child_model, finders);
// let child_node = graph.create_node(child_read_query);

// graph.create_edge(&parent_node, &child_node, QueryGraphDependency::ExecutionOrder)?;
// // connect::connect_records_node(graph, &parent_node, &child_node, &parent_relation_field, None, None)?;

// let set = WriteQuery::SetRecords(SetRecords {
//     parent: None,
//     wheres: vec![],
//     relation_field: Arc::clone(&parent_relation_field),
// });

// let set_node = graph.create_node(Query::Write(set));

// // Edge from parent to set.
// graph.create_edge(
//     &parent_node,
//     &set_node,
//     QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
//         let len = parent_ids.len();
//         if len == 0 {
//             Err(QueryGraphBuilderError::AssertionError(format!(
//                 "Required exactly one parent ID to be present for set query, found none."
//             )))
//         } else if len > 1 {
//             Err(QueryGraphBuilderError::AssertionError(format!(
//                 "Required exactly one parent ID to be present for set query, found {}.",
//                 len
//             )))
//         } else {
//             if let Node::Query(Query::Write(WriteQuery::SetRecords(ref mut x))) = child_node {
//                 let parent_id = parent_ids.pop().unwrap();
//                 x.parent = Some(parent_id.try_into()?);
//             }

//             Ok(child_node)
//         }
//     })),
// )?;

// // Edge from child to set.
// graph.create_edge(
//     &child_node,
//     &set_node,
//     QueryGraphDependency::ParentIds(Box::new(|mut child_node, parent_ids| {
//         if let Node::Query(Query::Write(WriteQuery::SetRecords(ref mut x))) = child_node {
//             x.wheres = parent_ids
//                 .iter()
//                 .map(|x| x.try_into().expect("Prisma Value was not a GraphqlId"))
//                 .collect();
//         }

//         Ok(child_node)
//     })),
// )?;

// Ok(())
