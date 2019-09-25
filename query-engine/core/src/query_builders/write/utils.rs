use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::{filter::RecordFinder, QueryArguments};
use prisma_models::{ModelRef, RelationFieldRef, SelectedFields};
use std::{convert::TryInto, sync::Arc};

/// Detects and performs a flip of `parent` and `child`, if necessary.
/// If a flip is performed: Removes all edges from the parent to it's parents, and rewire them to the child.
/// Note: Any edge existing between parent and child are NOT FLIPPED here.
///
/// Returns (parent `NodeRef`, child `NodeRef`).
pub fn flip_nodes<'a>(
    graph: &mut QueryGraph,
    parent: &'a NodeRef,
    child: &'a NodeRef,
    relation_field: &RelationFieldRef,
) -> (&'a NodeRef, &'a NodeRef) {
    let parent_node_content = graph.node_content(parent).unwrap();

    if let Node::Query(Query::Write(WriteQuery::CreateRecord(_))) = parent_node_content {
        if relation_field.relation_is_inlined_in_parent() {
            let parent_edges = graph.incoming_edges(parent);
            for parent_edge in parent_edges {
                let parent_of_parent_node = graph.edge_source(&parent_edge);
                let edge_content = graph.remove_edge(parent_edge).unwrap();

                // Todo: Warning, this assumes the edge contents can also be "flipped".
                graph.create_edge(&parent_of_parent_node, child, edge_content);
            }

            (child, parent)
        } else {
            (parent, child)
        }
    } else {
        (parent, child)
    }
}

/// Coerces single values (`ParsedInputValue::Single` and `ParsedInputValue::Map`) into a vector.
/// Simply unpacks `ParsedInputValue::List`.
pub fn coerce_vec(val: ParsedInputValue) -> Vec<ParsedInputValue> {
    match val {
        ParsedInputValue::List(l) => l,
        m @ ParsedInputValue::Map(_) => vec![m],
        single => vec![single],
    }
}

/// Produces a non-failing ReadQuery for a given RecordFinder by using
/// a ManyRecordsQuery instead of a find one (i.e. returns empty list instead of "not found" error).
pub fn id_read_query_infallible(model: &ModelRef, record_finder: RecordFinder) -> Query {
    let selected_fields: SelectedFields = model.fields().id().into();
    let read_query = ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
        name: "".into(),
        alias: None,
        model: Arc::clone(&model),
        args: record_finder.into(),
        selected_fields,
        nested: vec![],
        selection_order: vec![],
    });

    Query::Read(read_query)
}

/// Adds a read query to the query graph that finds related records by parent ID.
/// Connects the parent node and the read node witha an edge.
/// Returns a `NodeRef` to the newly created read node.
pub fn find_ids_by_parent(graph: &mut QueryGraph, relation_field: &RelationFieldRef, parent: &NodeRef) -> NodeRef {
    let read_parent_node = graph.create_node(Query::Read(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
        name: "parent".to_owned(),
        alias: None,
        parent_field: Arc::clone(relation_field),
        parent_ids: None,
        args: QueryArguments::default(),
        selected_fields: relation_field.related_model().fields().id().into(),
        nested: vec![],
        selection_order: vec![],
    })));

    graph.create_edge(
        parent,
        &read_parent_node,
        QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
            if let Node::Query(Query::Read(ReadQuery::RelatedRecordsQuery(ref mut rq))) = node {
                rq.parent_ids = Some(vec![parent_id.unwrap().try_into().unwrap()]);
            };

            node
        })),
    );

    read_parent_node
}
