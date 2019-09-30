use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::{filter::RecordFinder, QueryArguments};
use prisma_models::{ModelRef, RelationFieldRef, SelectedFields};
use std::{convert::TryInto, sync::Arc};

/// Detects and performs a flip of `parent` and `child` nodes, if necessary, which is basically a transformation
/// on the query graph to allow "incorrect" incoming queries to be executed.
///
/// When is a node flip necessary? If `child` is a create and the parent holds the inlined relation field,
/// e.g. the foreign key in SQL terms. This means we can't create the parent without knowing the actual
/// ID first. Hence, we need to execute the child node first to get the child ID, then the
/// parent node can execute. How the flipped nodes are connected in the end is the callers to decide.
///
/// Performing a flip involves:
/// - Removing all edges from the parent to it's parents
/// - Rewiring the removed edges to the child.
///
/// Note: Any edge already existing between parent and child are NOT FLIPPED here.
///
/// Returns the correct `RelationFieldRef` in the result triple. The relation field is always the one on the parent,
/// not the child, and flipping parent and child "flips" the relation field the code is reasoning about as well,
/// which is why we need to also return another relation field in case a flip happened.
/// Todo: This unfortunately requires us to clone the arcs to satisfy the interface. Any better solution possible?
///
/// Returns (parent `NodeRef`, child `NodeRef`, relation field on parent `RelationFieldRef`).
pub fn flip_nodes<'a>(
    graph: &mut QueryGraph,
    parent: &'a NodeRef,
    child: &'a NodeRef,
    relation_field: &'a RelationFieldRef,
) -> (&'a NodeRef, &'a NodeRef, RelationFieldRef) {
    if node_is_create(graph, child) {
        if relation_field.relation_is_inlined_in_parent() {
            let parent_edges = graph.incoming_edges(parent);
            for parent_edge in parent_edges {
                let parent_of_parent_node = graph.edge_source(&parent_edge);
                let edge_content = graph.remove_edge(parent_edge).unwrap();

                // Todo: Warning, this assumes the edge contents can also be "flipped".
                graph.create_edge(&parent_of_parent_node, child, edge_content);
            }

            (child, parent, relation_field.related_field())
        } else {
            (parent, child, Arc::clone(relation_field))
        }
    } else {
        (parent, child, Arc::clone(relation_field))
    }
}

pub fn node_is_create(graph: &QueryGraph, node: &NodeRef) -> bool {
    match graph.node_content(node).unwrap() {
        Node::Query(Query::Write(WriteQuery::CreateRecord(_))) => true,
        _ => false,
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
/// Connects the parent node and the read node with an edge, which takes care of the
/// node transformation based on the parent ID.
///
/// Optionally, a filter can be passed that narrows down the child selection.
///
/// Returns a `NodeRef` to the newly created read node.
pub fn find_ids_by_parent_node<T>(
    graph: &mut QueryGraph,
    relation_field: &RelationFieldRef,
    parent: &NodeRef,
    filter: T,
) -> NodeRef
where
    T: Into<QueryArguments>,
{
    let read_parent_node = graph.create_node(Query::Read(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
        name: "parent".to_owned(),
        alias: None,
        parent_field: Arc::clone(relation_field),
        parent_ids: None,
        args: filter.into(),
        selected_fields: relation_field.related_model().fields().id().into(),
        nested: vec![],
        selection_order: vec![],
    })));

    graph.create_edge(
        parent,
        &read_parent_node,
        QueryGraphDependency::ParentIds(Box::new(|mut node, parent_ids| {
            if let Node::Query(Query::Read(ReadQuery::RelatedRecordsQuery(ref mut rq))) = node {
                // We know that all PrismaValues in `parent_ids` are transformable into GraphqlIds.
                rq.parent_ids = Some(parent_ids.into_iter().map(|id| id.try_into().unwrap()).collect());
            };

            Ok(node)
        })),
    );

    read_parent_node
}

pub fn ensure_connected(graph: &mut QueryGraph) -> NodeRef {
    // load all children with their parent ids and make sure they match?
    unimplemented!()
}

/// Creates an "empty" query node. Sometimes required for
/// Todo: Consider elevating the placeholder concept to the actual graph.
/// - Prevents accidential reads, could just error if placeholder hasn't been replaced during building.
/// - Definitely the cleaner solution.
pub fn query_node_placeholder(graph: &mut QueryGraph) -> NodeRef {
    graph.create_node(Query::Read(ReadQuery::RecordQuery(RecordQuery::default())))
}
