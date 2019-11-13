use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, ParentIdsFn, QueryGraph, QueryGraphDependency},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use prisma_models::RelationFieldRef;
use std::{convert::TryInto, sync::Arc};

/// Only for many to many relations.
///
/// Creates a connect node in the graph and creates edges to `parent_node` and `child_node`.
/// By default, the connect node assumes that both the parent and the child node results
/// are convertible to IDs, as the edges perform a transformation on the connect node to
/// inject the required IDs after the parents executed.
///
/// Optionally, `ParentIdsFn`s can be provided to override the default edges for parent and child, respectively.
///
/// The resulting graph (dashed indicates that those nodes and edges are not created in this function):
///
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
/// └─▶│     Connect     │
///    └─────────────────┘
/// ```
pub fn connect_records_node(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    child_node: &NodeRef,
    relation_field: &RelationFieldRef,
    parent_fn: Option<ParentIdsFn>,
    child_fn: Option<ParentIdsFn>,
) -> QueryGraphBuilderResult<NodeRef> {
    assert!(relation_field.relation().is_many_to_many());

    let connect = WriteQuery::ConnectRecords(ConnectRecords {
        parent_id: None,
        child_ids: vec![],
        relation_field: Arc::clone(relation_field),
    });

    let connect_node = graph.create_node(Query::Write(connect));

    // Edge from parent to connect.
    graph.create_edge(
        parent_node,
        &connect_node,
        QueryGraphDependency::ParentIds(parent_fn.unwrap_or_else(|| {
            Box::new(|mut child_node, mut parent_ids| {
                let len = parent_ids.len();

                if len == 0 {
                    return Err(QueryGraphBuilderError::AssertionError(format!(
                        "Required exactly one parent ID to be present for connect query, found 0."
                    )));
                }

                if let Node::Query(Query::Write(WriteQuery::ConnectRecords(ref mut c))) = child_node {
                    let parent_id = parent_ids.pop().unwrap();
                    c.parent_id = Some(parent_id.try_into()?);
                }

                Ok(child_node)
            })
        })),
    )?;

    // Edge from child to connect.
    graph.create_edge(
        &child_node,
        &connect_node,
        QueryGraphDependency::ParentIds(child_fn.unwrap_or_else(|| {
            Box::new(|mut child_node, parent_ids| {
                let len = parent_ids.len();

                if len == 0 {
                    return Err(QueryGraphBuilderError::AssertionError(format!(
                        "Required one or more child IDs to be present for connect query, found 0."
                    )));
                }

                if let Node::Query(Query::Write(WriteQuery::ConnectRecords(ref mut c))) = child_node {
                    c.child_ids = parent_ids.into_iter().map(|id| id.try_into().unwrap()).collect();
                }

                Ok(child_node)
            })
        })),
    )?;

    Ok(connect_node)
}
