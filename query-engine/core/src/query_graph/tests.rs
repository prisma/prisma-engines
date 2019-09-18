use super::*;
use crate::{Query, ReadQuery, RecordQuery};

#[test]
fn test_direct_children() {
    let mut graph = QueryGraph::new();

    let grandparent = graph.create_node(dummy_query());
    let parent = graph.create_node(dummy_query());
    let child = graph.create_node(dummy_query());

    let edge_gp_p = graph.create_edge(&grandparent, &parent, QueryGraphDependency::ExecutionOrder);
    let edge_p_c = graph.create_edge(&parent, &child, QueryGraphDependency::ExecutionOrder);

    let g_children = graph.direct_child_pairs(&grandparent);
    let p_children = graph.direct_child_pairs(&parent);

    assert_eq!(g_children, vec![(edge_gp_p, parent)]);
    assert_eq!(p_children, vec![(edge_p_c, child)]);
}

#[test]
fn test_valid_multiparent() {
    let mut graph = QueryGraph::new();

    let grandparent = graph.create_node(dummy_query());
    let parent = graph.create_node(dummy_query());
    let child = graph.create_node(dummy_query());

    graph.create_edge(&grandparent, &parent, QueryGraphDependency::ExecutionOrder);
    graph.create_edge(&parent, &child, QueryGraphDependency::ExecutionOrder);

    // This must succeed
    graph.create_edge(&grandparent, &child, QueryGraphDependency::ExecutionOrder);
}

#[should_panic]
#[test]
fn test_invalid_multiparent() {
    let mut graph = QueryGraph::new();

    let parent_a = graph.create_node(dummy_query());
    let parent_b = graph.create_node(dummy_query());
    let child = graph.create_node(dummy_query());

    graph.create_edge(&parent_a, &child, QueryGraphDependency::ExecutionOrder);

    // This must fail
    graph.create_edge(&parent_b, &child, QueryGraphDependency::ExecutionOrder);
}

fn dummy_query() -> Query {
    Query::Read(ReadQuery::RecordQuery(RecordQuery::default()))
}
