///! Skeleton for QueryGraph validation rules.
///! Only basic POC rules, to be expanded later.
use super::*;

/// Check validity of an edge creation.
pub fn after_edge_creation(graph: &QueryGraph, edge: &EdgeRef) -> QueryGraphResult<()> {
    if_flow_edge_rules(graph, edge)
}

/// Only allow `Then` and `Else`. Disallow more than 2 edges.
fn if_flow_edge_rules(graph: &QueryGraph, edge: &EdgeRef) -> QueryGraphResult<()> {
    let source_node = graph.edge_source(edge);
    let source_node_content = graph.node_content(&source_node).unwrap();

    if let Node::Flow(Flow::If(_)) = source_node_content {
        match graph.edge_content(edge).unwrap() {
            QueryGraphDependency::Then | QueryGraphDependency::Else => Ok(()),
            x => Err(QueryGraphError::RuleViolation(format!(
                "Invalid edge '{}' for If node {}.",
                x,
                source_node.id()
            ))),
        }?;

        let pairs = graph.child_pairs(&source_node);
        if pairs.len() > 2 {
            return Err(QueryGraphError::RuleViolation(
                "'If' node has invalid amound of children (min 1, max 2).".into(),
            ));
        }
    }

    Ok(())
}
