use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::filter::RecordFinder;
use prisma_models::{PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Disconnects record IDs retrieved from `parent_node` and `child_node`.
/// Adds a disconnect query node to the graph, together with required edges.
///
/// A disconnect distinguishes between two cases:
/// - Relation is many-to-many: Delete extra record (e.g. in a join table), `WriteQuery::Disconnect` query.
/// - Relation is not many-to-many: Unset the inlined relation field on the side it is set. `WriteQuery::UpdateRecord` query.
///
/// Performs checks to make sure relations are not violated by the disconnect.
pub fn disconnect_records_node(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    child_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let relation = parent_relation_field.relation();

    if relation.is_many_to_many() {
        handle_many_to_many(graph, parent_node, child_node, parent_relation_field)
    } else {
        handle_one_to_x(graph, parent_node, child_node, parent_relation_field)
    }
}

/// Handles a many to many disconnect of `Parent` and `Child`.
/// Assumes that `Parent` and `Child` return IDs.
///
/// ## Graph
/// - Adds `Disconnect` to the given nodes.
/// - Illustration assumes that `Parent` and `Child` are connected in some way: The edge is not created in this function.
///
/// ```text
///    ┌────────────────┐
/// ┌──│     Parent     │
/// │  └────────────────┘
/// │           │
/// │           ▼
/// │  ┌────────────────┐
/// │  │     Child      │
/// │  └────────────────┘
/// │           │
/// │           ▼
/// │  ┌────────────────┐
/// └─▶│   Disconnect   │
///    └────────────────┘
/// ```
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    child_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let disconnect = WriteQuery::DisconnectRecords(DisconnectRecords {
        parent: None,
        child: None,
        relation_field: Arc::clone(parent_relation_field),
    });

    let disconnect_node = graph.create_node(Query::Write(disconnect));

    // Edge from parent to disconnect.
    graph.create_edge(
        parent_node,
        &disconnect_node,
        QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!(
                    "[Query Graph] Expected a valid parent ID to be present for a nested disconnect on a many-to-many relation."
                ))),
            }?;

            if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = child_node {
                c.parent = Some(parent_id.try_into()?);
            }

            Ok(child_node)
        })),
    )?;

    let relation_name = parent_relation_field.relation().name.clone();
    let parent_name = parent_relation_field.model().name.clone();
    let child_name = parent_relation_field.related_model().name.clone();

    // Edge from child to disconnect.
    graph.create_edge(
        &child_node,
        &disconnect_node,
        QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
            let len = parent_ids.len();
            if len == 0 {
                Err(QueryGraphBuilderError::RecordsNotConnected {
                    relation_name,
                    parent_name,
                    child_name,
                })
            } else if len > 1 {
                Err(QueryGraphBuilderError::AssertionError(format!(
                    "Required exactly one child ID to be present for connect query, found {}.",
                    len
                )))
            } else {
                if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = child_node {
                    let child_id = parent_ids.pop().unwrap();
                    c.child = Some(child_id.try_into()?);
                }

                Ok(child_node)
            }
        })),
    )?;

    Ok(disconnect_node)
}

/// Handles a one to many or one to one disconnect.
/// Depending on where the relation is inlined, an update node will be inserted:
/// ```text
/// Inlined on child:        Inlined on parent:
/// ┌────────────────┐       ┌────────────────┐
/// │     Child      │       │     Parent     │
/// └────────────────┘       └────────────────┘
///          │                        │
///          ▼                        ▼
/// ┌────────────────┐       ┌────────────────┐
/// │  Update Child  │       │ Update Parent  │
/// └────────────────┘       └────────────────┘
/// ```
///
/// Assumes that both `Parent` and `Child` return IDs.
/// We need to check that _both_ actually do return IDs to ensure that they're connected,
/// regardless of which ID is used in the end to perform the update.`
fn handle_one_to_x(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    child_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let child_relation_field = parent_relation_field.related_field();

    // If we're in a 1:m scenario and either relation side is required, a disconnect is impossible, as some
    // relation requirement would be violated with the disconnect.
    if parent_relation_field.is_required || child_relation_field.is_required {
        return Err(QueryGraphBuilderError::RelationViolation(parent_relation_field.into()));
    }

    // Depending on where the relation is inlined, we update the parent or the child and check the other one for ID presence.
    let (node_to_attach, node_to_check, model_to_update, relation_field_name, id_field) =
        if parent_relation_field.relation_is_inlined_in_parent() {
            let parent_model = parent_relation_field.model();
            let relation_field_name = parent_relation_field.name.clone();
            let parent_model_id = parent_model.fields().id();

            (
                parent_node,
                child_node,
                parent_model,
                relation_field_name,
                parent_model_id,
            )
        } else {
            let child_model = child_relation_field.model();
            let relation_field_name = child_relation_field.name.clone();
            let child_model_id = child_model.fields().id();

            (
                child_node,
                parent_node,
                child_model,
                relation_field_name,
                child_model_id,
            )
        };

    let update_node = utils::update_record_node_placeholder(graph, None, model_to_update);
    let relation_name = parent_relation_field.relation().name.clone();
    let parent_name = parent_relation_field.model().name.clone();
    let child_name = parent_relation_field.related_model().name.clone();

    graph.create_edge(
        node_to_attach,
        &update_node,
        QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::RecordsNotConnected {
                    relation_name,
                    parent_name,
                    child_name,
                }),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = child_node {
                wq.inject_non_list_arg(relation_field_name, PrismaValue::Null);
                wq.inject_record_finder(RecordFinder {
                    field: id_field,
                    value: parent_id,
                });
            }

            Ok(child_node)
        })),
    )?;

    let relation_name = parent_relation_field.relation().name.clone();
    let parent_name = parent_relation_field.model().name.clone();
    let child_name = parent_relation_field.related_model().name.clone();

    graph.create_edge(
        node_to_check,
        &update_node,
        QueryGraphDependency::ParentIds(Box::new(|child_node, parent_ids| {
            if parent_ids.is_empty() {
                return Err(QueryGraphBuilderError::RecordsNotConnected {
                    relation_name,
                    parent_name,
                    child_name,
                });
            }

            Ok(child_node)
        })),
    )?;

    Ok(update_node)
}
