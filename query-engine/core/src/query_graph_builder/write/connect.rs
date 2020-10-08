use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use prisma_models::RelationFieldRef;
use std::sync::Arc;

/// Only for many to many relations.
///
/// Creates a connect node in the graph and creates edges to `parent_node` and `child_node`.
/// By default, the connect node assumes that both the parent and the child node results
/// are convertible to IDs, as the edges perform a transformation on the connect node to
/// inject the required IDs after the parents executed.
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
    parent_relation_field: &RelationFieldRef,
    expected_connects: usize,
) -> QueryGraphBuilderResult<NodeRef> {
    assert!(parent_relation_field.relation().is_many_to_many());

    let parent_model_id = parent_relation_field.model().primary_identifier();
    let child_model_id = parent_relation_field.related_model().primary_identifier();

    let connect = WriteQuery::ConnectRecords(ConnectRecords {
        parent_id: None,
        child_ids: vec![],
        relation_field: Arc::clone(parent_relation_field),
    });

    let connect_node = graph.create_node(Query::Write(connect));

    // Edge from parent to connect.
    graph.create_edge(
        parent_node,
        &connect_node,
        QueryGraphDependency::ParentProjection(
            parent_model_id,
            Box::new(|mut connect_node, mut parent_ids| {
                let len = parent_ids.len();

                if len == 0 {
                    return Err(QueryGraphBuilderError::AssertionError("Required exactly one parent ID to be present for connect query, found 0.".to_string()));
                }

                if let Node::Query(Query::Write(WriteQuery::ConnectRecords(ref mut c))) = connect_node {
                    let parent_id = parent_ids.pop().unwrap();
                    c.parent_id = Some(parent_id);
                }

                Ok(connect_node)
            }),
        ),
    )?;

    // Edge from child to connect.
    graph.create_edge(
        &child_node,
        &connect_node,
        QueryGraphDependency::ParentProjection(
            child_model_id,
            Box::new(move |mut connect_node, child_ids| {
                let len = child_ids.len();

                if len != expected_connects {
                    return Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "Expected {} records to be connected, found only {}.",
                        expected_connects, len,
                    )));
                }

                if let Node::Query(Query::Write(WriteQuery::ConnectRecords(ref mut c))) = connect_node {
                    c.child_ids = child_ids;
                }

                Ok(connect_node)
            }),
        ),
    )?;

    Ok(connect_node)
}
