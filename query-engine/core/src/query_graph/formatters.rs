use super::*;
use std::fmt::{self, Display};

pub fn format(graph: &QueryGraph) -> String {
    format!(
        "---- Query Graph ----\nResult Nodes: {:?}\n\n{}\n----------------------",
        graph.result_nodes,
        stringify_nodes(graph, graph.root_nodes()).join("\n\n")
    )
}

fn stringify_nodes(graph: &QueryGraph, nodes: Vec<NodeRef>) -> Vec<String> {
    let mut rendered_nodes = vec![];

    for node in nodes {
        let mut node_child_info = vec![];

        let children: Vec<NodeRef> = graph
            .outgoing_edges(&node)
            .iter()
            .map(|child_edge| {
                let child_node = graph.edge_target(child_edge);
                node_child_info.push(format!(
                    "Child (edge {}): Node {} - {}",
                    child_edge.id(),
                    child_node.id(),
                    graph.edge_content(child_edge).unwrap()
                ));

                child_node
            })
            .collect();

        rendered_nodes.push(format!(
            "Node {}: {}\n  {}",
            node.id(),
            graph.node_content(&node).unwrap(),
            node_child_info.join("\n  ")
        ));

        rendered_nodes.append(&mut stringify_nodes(graph, children));
    }

    rendered_nodes
}

impl Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::If(_) => write!(f, "(If (condition func)"),
        }
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Query(q) => write!(f, "{}", q),
            Self::Flow(flow) => write!(f, "{}", flow),
        }
    }
}

impl Display for QueryGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format(self))
    }
}

impl Display for QueryGraphDependency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ExecutionOrder => write!(f, "ExecutionOrder"),
            Self::ParentIds(_) => write!(f, "ParentIds"),
            Self::Then => write!(f, "Then"),
            Self::Else => write!(f, "Else"),
        }
    }
}
