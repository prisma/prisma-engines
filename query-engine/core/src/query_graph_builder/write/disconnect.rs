use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    DataExpectation, DataOperation, MissingRelatedRecord, QueryGraphBuilderResult,
};
use query_structure::RelationFieldRef;

/// Only for many to many relations.
///
/// Creates a disconnect node in the graph and creates edges to `parent_node` and `child_node`.
/// By default, the connect node assumes that both the parent and the child node results
/// are convertible to IDs, as the edges perform a transformation on the connect node to
/// inject the required IDs after the parents executed.
///
/// The resulting graph (dashed indicates that those nodes and edges are not created in this function):
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
pub(crate) fn disconnect_records_node(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    child_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<NodeRef> {
    assert!(parent_relation_field.relation().is_many_to_many());

    let parent_model_id = parent_relation_field.model().shard_aware_primary_identifier();
    let child_model_id = parent_relation_field.related_model().shard_aware_primary_identifier();

    let disconnect = WriteQuery::DisconnectRecords(DisconnectRecords {
        parent_id: None,
        child_ids: vec![],
        relation_field: parent_relation_field.clone(),
    });

    let disconnect_node = graph.create_node(Query::Write(disconnect));

    // Edge from parent to disconnect.
    graph.create_edge(
        parent_node,
        &disconnect_node,
        QueryGraphDependency::ProjectedDataDependency(
            parent_model_id,
            Box::new(move |mut disconnect_node, mut parent_ids| {
                let parent_id = parent_ids.pop().expect("parent id should be present");

                if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = disconnect_node {
                    c.parent_id = Some(parent_id);
                }

                Ok(disconnect_node)
            }),
            Some(DataExpectation::non_empty_rows(
                MissingRelatedRecord::builder()
                    .model(&parent_relation_field.model())
                    .relation(&parent_relation_field.relation())
                    .operation(DataOperation::Disconnect)
                    .build(),
            )),
        ),
    )?;

    // Edge from child to disconnect.
    graph.create_edge(
        child_node,
        &disconnect_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_id,
            Box::new(move |mut disconnect_node, child_ids| {
                if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = disconnect_node {
                    c.child_ids = child_ids;
                }

                Ok(disconnect_node)
            }),
            None,
        ),
    )?;

    Ok(disconnect_node)
}
