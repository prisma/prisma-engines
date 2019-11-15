use super::*;
use crate::{
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue, Query, RecordFinderInjector, WriteQuery,
};
use connector::Filter;
use itertools::Itertools;
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::convert::TryInto;

pub fn connect_nested_disconnect(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    if relation.is_many_to_many() {
        // Build all finders upfront.
        let finders: Vec<RecordFinder> = utils::coerce_vec(value)
            .into_iter()
            .map(|value: ParsedInputValue| extract_record_finder(value, &child_model))
            .collect::<QueryGraphBuilderResult<Vec<RecordFinder>>>()?
            .into_iter()
            .unique()
            .collect();

        handle_many_to_many(graph, &parent_node, parent_relation_field, finders)
    } else {
        let finders: Vec<RecordFinder> = if relation.is_one_to_one() {
            // One-to-one relations simply specify if they want to disconnect the child or not as a bool.
            let val: PrismaValue = value.try_into()?;
            let should_delete = if let PrismaValue::Boolean(b) = val { b } else { false };

            if !should_delete {
                return Ok(());
            }

            vec![]
        } else {
            // One-to-many specify a number of finders if the parent side is the to-one.
            // todo check if this if else is really still required.
            if parent_relation_field.is_list {
                utils::coerce_vec(value)
                    .into_iter()
                    .map(|value: ParsedInputValue| extract_record_finder(value, &child_model))
                    .collect::<QueryGraphBuilderResult<Vec<RecordFinder>>>()?
                    .into_iter()
                    .unique()
                    .collect()
            } else {
                vec![]
            }
        };

        handle_one_to_x(graph, &parent_node, parent_relation_field, finders)
    }
}

/// Handles a nested many-to-many disconnect.
///
/// Creates a disconnect node in the graph and creates edges to `parent_node` and `child_node`.
/// The disconnect edges assume that both the parent and the child node results
/// are convertible to IDs, as the edges perform a transformation on the disconnect node to
/// inject the required IDs after the parents executed.
///
/// The resulting graph:
/// (dashed indicates that those nodes and edges are not created in this function)
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent       ─ ─ ─ ─ ─
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │
/// │           │
/// │                              │
/// │           │
/// │           ▼                  ▼
/// │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐  ┌ ─ ─ ─ ─ ─ ─
/// │         Child             Result   │
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘  └ ─ ─ ─ ─ ─ ─
/// │           │
/// │           │
/// │           │
/// │           ▼
/// │  ┌─────────────────┐
/// └─▶│   Disconnect    │
///    └─────────────────┘
/// ```
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    finders: Vec<RecordFinder>,
) -> QueryGraphBuilderResult<()> {
    let expected_disconnects = std::cmp::max(finders.len(), 1);
    let filter: Filter = finders.into();
    let find_child_records_node =
        utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, filter)?;

    disconnect::disconnect_records_node(
        graph,
        parent_node,
        &find_child_records_node,
        &parent_relation_field,
        expected_disconnects,
    )?;
    Ok(())
}

/// Handles a nested one to many or one to one disconnect.
///
/// Depending on where the relation is inlined, an update node will be inserted:
/// (dashed indicates that those nodes and edges are not created in this function)
/// ```text
/// Inlined on parent:        Inlined on child:
///
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐            ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent                ┌──      Parent
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │           │                  │           │
/// │           │             Fail if !=       │
/// │           │              expected        │
/// │           ▼                  │           ▼
/// │  ┌─────────────────┐         │  ┌─────────────────┐
/// │  │  Find Children  │         │  │  Find Children  │
/// │  └─────────────────┘         │  └─────────────────┘
/// │      Fail if !=              │           │
/// │       expected               │           │
/// │           │                  │           │
/// │           ▼                  │           ▼
/// │  ┌─────────────────┐         │  ┌─────────────────┐
/// └─▶│  Update Parent  │         └─▶│ Update Children │
///    └─────────────────┘            └─────────────────┘
/// ```
///
/// Assumes that both `Parent` and `Child` return IDs.
/// We need to check that _both_ actually do return IDs to ensure that they're connected,
/// regardless of which ID is used in the end to perform the update.
///
/// Todo pretty sure it's better do redo this code with separate handlers.
fn handle_one_to_x(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    finders: Vec<RecordFinder>,
) -> QueryGraphBuilderResult<()> {
    let finders_len = finders.len();

    // Fetches children to be disconnected.
    let find_child_records_node =
        utils::insert_find_children_by_parent_node(graph, &parent_node, parent_relation_field, finders)?;

    let child_relation_field = parent_relation_field.related_field();

    // If we're in a 1:m scenario and either relation side is required, a disconnect is impossible, as some
    // relation requirement would be violated with the disconnect.
    if parent_relation_field.is_required || child_relation_field.is_required {
        return Err(QueryGraphBuilderError::RelationViolation(parent_relation_field.into()));
    }

    // Depending on where the relation is inlined, we update the parent or the child and check the other one for ID presence.
    let (node_to_attach, node_to_check, model_to_update, relation_field_name, id_field, expected_disconnects) =
        if parent_relation_field.relation_is_inlined_in_parent() {
            let parent_model = parent_relation_field.model();
            let relation_field_name = parent_relation_field.name.clone();
            let parent_model_id = parent_model.fields().id();

            (
                parent_node,
                &find_child_records_node,
                parent_model,
                relation_field_name,
                parent_model_id,
                std::cmp::max(finders_len, 1),
            )
        } else {
            let child_model = child_relation_field.model();
            let relation_field_name = child_relation_field.name.clone();
            let child_model_id = child_model.fields().id();

            (
                &find_child_records_node,
                parent_node,
                child_model,
                relation_field_name,
                child_model_id,
                1,
            )
        };

    let update_node = utils::update_records_node_placeholder(graph, None, model_to_update);
    let relation_name = parent_relation_field.relation().name.clone();
    let parent_name = parent_relation_field.model().name.clone();
    let child_name = parent_relation_field.related_model().name.clone();

    // Edge to inject the correct data into the update (either from the parent or child).
    graph.create_edge(
        node_to_attach,
        &update_node,
        QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
            if parent_ids.len() == 0 {
                return Err(QueryGraphBuilderError::RecordsNotConnected {
                    relation_name,
                    parent_name,
                    child_name,
                });
            }

            // Handle finder / filter injection
            match child_node {
                Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) => {
                    ur.filter = parent_ids
                        .into_iter()
                        .map(|id| RecordFinder {
                            field: id_field.clone(),
                            value: id,
                        })
                        .collect::<Vec<RecordFinder>>()
                        .into()
                }

                Node::Query(Query::Write(ref mut wq)) => wq.inject_record_finder(RecordFinder {
                    field: id_field,
                    value: parent_ids.pop().unwrap(),
                }),
                _ => unimplemented!(),
            };

            // Handle arg injection
            if let Node::Query(Query::Write(ref mut wq)) = child_node {
                wq.inject_non_list_arg(relation_field_name, PrismaValue::Null);
            }

            Ok(child_node)
        })),
    )?;

    let relation_name = parent_relation_field.relation().name.clone();
    let parent_name = parent_relation_field.model().name.clone();
    let child_name = parent_relation_field.related_model().name.clone();

    // Edge to check that IDs have been returned.
    graph.create_edge(
        node_to_check,
        &update_node,
        QueryGraphDependency::ParentIds(Box::new(move |child_node, parent_ids| {
            if parent_ids.len() != expected_disconnects {
                return Err(QueryGraphBuilderError::RecordsNotConnected {
                    relation_name,
                    parent_name,
                    child_name,
                });
            }

            Ok(child_node)
        })),
    )?;

    Ok(())
}

// fn handle_one_to_x(
//     graph: &mut QueryGraph,
//     parent_node: &NodeRef,
//     child_node: &NodeRef,
//     parent_relation_field: &RelationFieldRef,
// ) -> QueryGraphBuilderResult<NodeRef> {
//     let child_relation_field = parent_relation_field.related_field();

//     // If we're in a 1:m scenario and either relation side is required, a disconnect is impossible, as some
//     // relation requirement would be violated with the disconnect.
//     if parent_relation_field.is_required || child_relation_field.is_required {
//         return Err(QueryGraphBuilderError::RelationViolation(parent_relation_field.into()));
//     }

//     // Depending on where the relation is inlined, we update the parent or the child and check the other one for ID presence.
//     let (node_to_attach, node_to_check, model_to_update, relation_field_name, id_field) =
//         if parent_relation_field.relation_is_inlined_in_parent() {
//             let parent_model = parent_relation_field.model();
//             let relation_field_name = parent_relation_field.name.clone();
//             let parent_model_id = parent_model.fields().id();

//             (
//                 parent_node,
//                 child_node,
//                 parent_model,
//                 relation_field_name,
//                 parent_model_id,
//             )
//         } else {
//             let child_model = child_relation_field.model();
//             let relation_field_name = child_relation_field.name.clone();
//             let child_model_id = child_model.fields().id();

//             (
//                 child_node,
//                 parent_node,
//                 child_model,
//                 relation_field_name,
//                 child_model_id,
//             )
//         };

//     let update_node = utils::update_records_node_placeholder(graph, None, model_to_update);
//     let relation_name = parent_relation_field.relation().name.clone();
//     let parent_name = parent_relation_field.model().name.clone();
//     let child_name = parent_relation_field.related_model().name.clone();

//     graph.create_edge(
//         node_to_attach,
//         &update_node,
//         QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
//             let parent_id = match parent_ids.pop() {
//                 Some(pid) => Ok(pid),
//                 None => Err(QueryGraphBuilderError::RecordsNotConnected {
//                     relation_name,
//                     parent_name,
//                     child_name,
//                 }),
//             }?;

//             if let Node::Query(Query::Write(ref mut wq)) = child_node {
//                 wq.inject_non_list_arg(relation_field_name, PrismaValue::Null);
//                 wq.inject_record_finder(RecordFinder {
//                     field: id_field,
//                     value: parent_id,
//                 });
//             }

//             Ok(child_node)
//         })),
//     )?;

//     let relation_name = parent_relation_field.relation().name.clone();
//     let parent_name = parent_relation_field.model().name.clone();
//     let child_name = parent_relation_field.related_model().name.clone();

//     graph.create_edge(
//         node_to_check,
//         &update_node,
//         QueryGraphDependency::ParentIds(Box::new(|child_node, parent_ids| {
//             if parent_ids.is_empty() {
//                 return Err(QueryGraphBuilderError::RecordsNotConnected {
//                     relation_name,
//                     parent_name,
//                     child_name,
//                 });
//             }

//             Ok(child_node)
//         })),
//     )?;

//     Ok(update_node)
// }

// fn handle_many_to_many(
//     graph: &mut QueryGraph,
//     parent_node: &NodeRef,
//     child_node: &NodeRef,
//     parent_relation_field: &RelationFieldRef,
// ) -> QueryGraphBuilderResult<NodeRef> {
//     let disconnect = WriteQuery::DisconnectRecords(DisconnectRecords {
//         parent_id: None,
//         child_ids: vec![],
//         relation_field: Arc::clone(parent_relation_field),
//     });

//     let disconnect_node = graph.create_node(Query::Write(disconnect));

//     // Edge from parent to disconnect.
//     graph.create_edge(
//         parent_node,
//         &disconnect_node,
//         QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
//             let parent_id = match parent_ids.pop() {
//                 Some(pid) => Ok(pid),
//                 None => Err(QueryGraphBuilderError::AssertionError(format!(
//                     "[Query Graph] Expected a valid parent ID to be present for a nested disconnect on a many-to-many relation."
//                 ))),
//             }?;

//             if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = child_node {
//                 c.parent_id = Some(parent_id.try_into()?);
//             }

//             Ok(child_node)
//         })),
//     )?;

//     let relation_name = parent_relation_field.relation().name.clone();
//     let parent_name = parent_relation_field.model().name.clone();
//     let child_name = parent_relation_field.related_model().name.clone();

//     // Edge from child to disconnect.
//     graph.create_edge(
//         &child_node,
//         &disconnect_node,
//         QueryGraphDependency::ParentIds(Box::new(|mut child_node, parent_ids| {
//             let len = parent_ids.len();

//             if len == 0 {
//                 return Err(QueryGraphBuilderError::RecordsNotConnected {
//                     relation_name,
//                     parent_name,
//                     child_name,
//                 });
//             }

//             if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = child_node {
//                 let child_ids = parent_ids.into_iter().map(|id| id.try_into().unwrap()).collect();
//                 c.child_ids = child_ids;
//             }

//             Ok(child_node)
//         })),
//     )?;

//     Ok(disconnect_node)
// }
