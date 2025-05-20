use super::*;
use std::fmt::{self, Display};

pub fn format(graph: &QueryGraph) -> String {
    let root_nodes: Vec<NodeRef> = graph.root_nodes().collect();
    format!(
        "---- Query Graph ----\nResult Nodes: {}\nMarked Nodes: {}\nRoot Nodes: {}\n\n{}\n----------------------",
        fmt_raw_indices(&graph.result_nodes),
        fmt_node_tuples(&graph.marked_node_pairs),
        fmt_node_list(&root_nodes),
        stringify_nodes(graph, root_nodes, &mut Vec::new()).join("\n\n")
    )
}

fn stringify_nodes(graph: &QueryGraph, nodes: Vec<NodeRef>, seen_nodes: &mut Vec<NodeRef>) -> Vec<String> {
    let mut rendered_nodes = vec![];

    for node in nodes {
        if seen_nodes.contains(&node) {
            continue;
        }

        seen_nodes.push(node);
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

        rendered_nodes.append(&mut stringify_nodes(graph, children, seen_nodes));
    }

    rendered_nodes
}

impl Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::If { rule, .. } => write!(f, "If {rule:?}"),
            Self::Return(_) => write!(f, "Return"),
        }
    }
}

impl Display for Computation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiffLeftToRight(_) => write!(f, "DiffLeftToRight"),
            Self::DiffRightToLeft(_) => write!(f, "DiffRightToLeft"),
        }
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Query(q) => write!(f, "{q}"),
            Self::Flow(flow) => write!(f, "{flow}"),
            Self::Computation(c) => write!(f, "{c}"),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

impl ToGraphviz for Node {
    fn to_graphviz(&self) -> String {
        match self {
            Node::Query(q) => q.to_graphviz(),
            Node::Flow(f) => format!("{f}"),
            Node::Computation(c) => format!("{c}"),
            Node::Empty => "Empty".to_string(),
        }
    }
}

impl Display for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node {}", self.id())
    }
}

impl Display for QueryGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format(self))
    }
}

impl Display for QueryGraphDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutionOrder => write!(f, "ExecutionOrder"),
            Self::DataDependency(_, _) => write!(f, "ParentResult"),
            Self::ProjectedDataDependency(selection, _, _) => {
                write!(
                    f,
                    "ProjectedDataDependency ({:?})",
                    selection
                        .selections()
                        .map(|f| format!("{}.{}", f.container().name(), f.prisma_name()))
                        .collect::<Vec<_>>()
                )
            }
            Self::ProjectedDataSinkDependency(selection, sink, _) => {
                write!(
                    f,
                    "ProjectedDataSinkDependency({sink:?}) {:?}",
                    selection
                        .selections()
                        .map(|f| format!("{}.{}", f.container().name(), f.prisma_name()))
                        .collect::<Vec<_>>()
                )
            }
            Self::Then => write!(f, "Then"),
            Self::Else => write!(f, "Else"),
        }
    }
}

fn fmt_raw_indices(i: &[NodeIndex]) -> String {
    let refs: Vec<NodeRef> = i.iter().map(|node_ix| NodeRef { node_ix: *node_ix }).collect();

    fmt_node_list(&refs)
}

fn fmt_node_list(v: &[NodeRef]) -> String {
    let inner_string = v.iter().map(|x| format!("{x}")).collect::<Vec<String>>().join(", ");

    format!("[{}]", inner_string.as_str())
}

fn fmt_node_tuples(t: &[(NodeRef, NodeRef)]) -> String {
    let inner_string = t
        .iter()
        .map(|x| format!("({}, {})", x.0, x.1))
        .collect::<Vec<String>>()
        .join(", ");

    format!("[{}]", inner_string.as_str())
}
