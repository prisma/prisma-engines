//! Skeleton for QueryGraph invariance rules checker.
//! Only basic POC rules jotted down at the moment, to be expanded later.
use super::*;
// use itertools::Itertools;

/// Check validity of an edge creation.
pub fn after_edge_creation(graph: &QueryGraph, edge: &EdgeRef) -> QueryGraphResult<()> {
    if_flow_edge_rules(graph, edge).and_then(|_| disallow_self_edges(graph, edge))
}

/// Check validity of graph, after graph is done building.
pub fn after_graph_completion(graph: &QueryGraph) -> QueryGraphResult<()> {
    only_allow_related_parents_edges(graph)
}

/// For an If flow node: Only allow `Then` and `Else` edges. Disallow more than 2 edges.
fn if_flow_edge_rules(graph: &QueryGraph, edge: &EdgeRef) -> QueryGraphResult<()> {
    let source_node = graph.edge_source(edge);
    let source_node_content = graph.node_content(&source_node).unwrap();

    if let Node::Flow(Flow::If(_)) = source_node_content {
        match graph.edge_content(edge).unwrap() {
            QueryGraphDependency::Then | QueryGraphDependency::Else => Ok(()),
            x => Err(QueryGraphError::InvarianceViolation(format!(
                "Invalid edge '{}' for If node {}.",
                x,
                source_node.id()
            ))),
        }?;

        let pairs = graph.child_pairs(&source_node);
        if pairs.len() > 2 {
            return Err(QueryGraphError::InvarianceViolation(
                "'If' node has invalid amound of children (min 1, max 2).".into(),
            ));
        }
    }

    Ok(())
}

fn disallow_self_edges(graph: &QueryGraph, edge: &EdgeRef) -> QueryGraphResult<()> {
    if graph.edge_source(edge).id() == graph.edge_target(edge).id() {
        return Err(QueryGraphError::InvarianceViolation(format!(
            "Edge {} is an edge pointing to the same node it originated from (node {}). This is disallowed.",
            edge.id(),
            graph.edge_source(edge).id()
        )));
    }

    Ok(())
}

/// Only allow multiple parent edges if all parents are ancestors of each other.
fn only_allow_related_parents_edges(_graph: &QueryGraph) -> QueryGraphResult<()> {
    // for edge in graph.edges() {
    //     let target_node = graph.edge_target(&edge);
    //     let incoming_edges = graph.incoming_edges(&target_node);
    //     let parents: Vec<NodeRef> = graph
    //         .zip_source_nodes(incoming_edges)
    //         .into_iter()
    //         .map(|(_, node)| node)
    //         .collect();

    //     let failed_parent_combination =
    //         dbg!(parents
    //             .iter()
    //             .tuple_combinations()
    //             .into_iter()
    //             .find(|(parent_a, parent_b)| {
    //                 !graph.is_ancestor(&parent_a, &parent_b) && !graph.is_ancestor(&parent_b, &parent_a)
    //             }));

    //     let check = if failed_parent_combination.is_none() {
    //         Ok(())
    //     } else {
    //         let unwrapped = failed_parent_combination.unwrap();
    //         Err(QueryGraphError::InvarianceViolation(format!(
    //             "Edge {} from node {} to node {} violates constraint that all parents must be ancestors of each other.",
    //             edge.id(),
    //             unwrapped.0.id(),
    //             unwrapped.1.id(),
    //         )))
    //     };

    //     check?;
    // }

    Ok(())
}
