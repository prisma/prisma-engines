use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, ParentIdsFn, QueryGraph, QueryGraphDependency},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use prisma_models::RelationFieldRef;
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
    relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let disconnect = WriteQuery::DisconnectRecords(DisconnectRecords {
        parent: None,
        child: None,
        relation_field: Arc::clone(relation_field),
    });

    let disconnect_node = graph.create_node(Query::Write(disconnect));

    // Edge from parent to disconnect.
    graph.create_edge(
        parent_node,
        &disconnect_node,
        QueryGraphDependency::ParentIds(|| {
            Box::new(|mut child_node, mut parent_ids| {
                let len = parent_ids.len();
                if len == 0 {
                    Err(QueryGraphBuilderError::AssertionError(format!(
                        "Required exactly one parent ID to be present for disconnect query, found none."
                    )))
                } else if len > 1 {
                    Err(QueryGraphBuilderError::AssertionError(format!(
                        "Required exactly one parent ID to be present for disconnect query, found {}.",
                        len
                    )))
                } else {
                    if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = child_node {
                        let parent_id = parent_ids.pop().unwrap();
                        c.parent = Some(parent_id.try_into()?);
                    }

                    Ok(child_node)
                }
            })
        }),
    )?;

    // Edge from child to disconnect.
    graph.create_edge(
        &child_node,
        &disconnect_node,
        QueryGraphDependency::ParentIds(child_fn.unwrap_or_else(|| {
            Box::new(|mut child_node, mut parent_ids| {
                let len = parent_ids.len();
                if len == 0 {
                    Err(QueryGraphBuilderError::AssertionError(format!(
                        "Required exactly one child ID to be present for connect query, found none."
                    )))
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
            })
        })),
    )?;

    Ok(disconnect_node)
}
