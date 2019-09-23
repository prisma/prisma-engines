use super::*;
use crate::{Query, ReadQuery, RecordQuery};

#[test]
fn test_direct_children() {
    let mut graph = QueryGraph::new();

    let grandparent = graph.create_node(dummy_query());
    let parent = graph.create_node(dummy_query());
    let child = graph.create_node(dummy_query());

    let edge_gp_p = graph.create_edge(&grandparent, &parent, QueryGraphDependency::ExecutionOrder);
    let _edge_gp_c = graph.create_edge(&grandparent, &child, QueryGraphDependency::ExecutionOrder);
    let edge_p_c = graph.create_edge(&parent, &child, QueryGraphDependency::ExecutionOrder);

    let g_children = graph.direct_child_pairs(&grandparent);
    let p_children = graph.direct_child_pairs(&parent);

    assert_eq!(g_children, vec![(edge_gp_p, parent)]);
    assert_eq!(p_children, vec![(edge_p_c, child)]);
}

#[test]
fn test_direct_children_2() {
    let mut graph = QueryGraph::new();

    let dummy_read = graph.create_node(dummy_query()); // r
    let dummy_create = graph.create_node(dummy_query()); // c
    let dummy_connect = graph.create_node(dummy_query()); // con
    let dummy_result = graph.create_node(dummy_query()); // res

    graph.add_result_node(&dummy_result);

    let edge_r_c = graph.create_edge(&dummy_read, &dummy_create, QueryGraphDependency::ExecutionOrder);
    let edge_c_res = graph.create_edge(&dummy_create, &dummy_result, QueryGraphDependency::ExecutionOrder);
    let _edge_r_con = graph.create_edge(&dummy_read, &dummy_connect, QueryGraphDependency::ExecutionOrder);
    let edge_c_con = graph.create_edge(&dummy_create, &dummy_connect, QueryGraphDependency::ExecutionOrder);

    let r_children = graph.direct_child_pairs(&dummy_read);
    let c_children = graph.direct_child_pairs(&dummy_create);

    assert_eq!(r_children, vec![(edge_r_c, dummy_create)]);
    assert_eq!(
        c_children,
        vec![(edge_c_res, dummy_result), (edge_c_con, dummy_connect)]
    );
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
