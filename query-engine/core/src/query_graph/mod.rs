///! Query graph abstraction for simple high-level query representation
///! and manipulation.
mod guard;

use connector::*;
use guard::*;
use petgraph::{graph::*, visit::EdgeRef, *};
use prisma_models::PrismaValue;
use std::{borrow::Borrow, collections::VecDeque};

/// Implementation detail of the QueryGraph.
type InnerGraph = Graph<Guard<Query>, Guard<QueryDependency>>;

#[derive(Default)]
pub struct QueryGraph {
    graph: InnerGraph,

    /// Designates the node that is returning the result of the entire QueryGraph.
    /// If no node is set, the interpretation will take the result of the
    /// last statement derived from the query graph.
    result_node: Option<NodeIndex>,
}

impl std::fmt::Display for QueryGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.stringify())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    pub node_ix: NodeIndex,
}

impl Node {
    /// Returns a unique node identifier.
    pub fn id(&self) -> String {
        self.node_ix.index().to_string()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    edge_ix: EdgeIndex,
}

/// Stored on the edges of the QueryGraph, a QueryDependency contains information on how children are connected to their parents,
/// expressing for example the need for additional information from the parent to be able to execute at runtime.
pub enum QueryDependency {
    /// Simple dependency indicating order of execution. Effectively a NOOP for now.
    ExecutionOrder,

    /// Performs a transformation on a query type T based on the parent ID (PrismaValue)
    ParentId(Box<dyn FnOnce(Query, PrismaValue) -> Query>),

    /// Expresses a conditional dependency that decides whether or not the child node
    /// is included in the execution.
    /// Currently, the evaluation function receives the parent ID as PrismaValue if it exists,
    /// None otherwise.
    Conditional(Box<dyn FnOnce(Option<PrismaValue>) -> bool>),
}

impl std::fmt::Display for QueryDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ExecutionOrder => write!(f, "ExecutionOrder"),
            Self::ParentId(_) => write!(f, "ParentId"),
            Self::Conditional(_) => write!(f, "Conditional"),
        }
    }
}

impl QueryGraph {
    pub fn new() -> Self {
        Self {
            graph: InnerGraph::new(),
            result_node: None,
        }
    }

    /// Sets the result node of the graph.
    pub fn set_result_node(&mut self, node: &Node) {
        self.result_node.replace(node.node_ix.clone());
    }

    /// Checks if the given node is marked as the result node in the graph.
    pub fn is_result_node(&self, node: &Node) -> bool {
        match self.result_node.borrow().as_ref() {
            Some(ix) => ix == &node.node_ix,
            None => false,
        }
    }

    /// Checks if the subgraph starting at the given node contains the node designated as the overall result.
    pub fn subgraph_contains_result(&self, node: &Node) -> bool {
        self.is_result_node(node)
            || self
                .outgoing_edges(node)
                .into_iter()
                .find(|edge| {
                    let child_node = self.edge_target(edge);
                    self.subgraph_contains_result(&child_node)
                })
                .is_some()
    }

    pub fn root_nodes(&self) -> Vec<Node> {
        let graph = self.graph.borrow();

        graph
            .node_indices()
            .filter_map(|ix| {
                if let Some(_) = graph.edges_directed(ix, Direction::Incoming).next() {
                    None
                } else {
                    Some(ix)
                }
            })
            .map(|node_ix: NodeIndex| Node { node_ix })
            .collect()
    }

    pub fn create_node(&mut self, query: Query) -> Node {
        let node_ix = self.graph.add_node(Guard::new(query));

        Node { node_ix }
    }

    pub fn create_edge(&mut self, from: &Node, to: &Node, content: QueryDependency) -> Edge {
        let edge_ix = self.graph.add_edge(from.node_ix, to.node_ix, Guard::new(content));

        Edge { edge_ix }
    }

    pub fn node_content(&self, node: &Node) -> Option<&Query> {
        self.graph.node_weight(node.node_ix).unwrap().borrow()
        // std::cell::Ref::map(self.graph.borrow(), |g| g.node_weight(node.node_ix).unwrap().borrow())
    }

    pub fn edge_content(&self, edge: &Edge) -> Option<&QueryDependency> {
        // std::cell::Ref::map(self.graph.borrow(), |g| g.edge_weight(edge.edge_ix).unwrap().borrow())
        self.graph.edge_weight(edge.edge_ix).unwrap().borrow()
    }

    pub fn edge_source(&self, edge: &Edge) -> Node {
        let (node_ix, _) = self.graph.edge_endpoints(edge.edge_ix).unwrap();

        Node { node_ix }
    }

    pub fn edge_target(&self, edge: &Edge) -> Node {
        let (_, node_ix) = self.graph.edge_endpoints(edge.edge_ix).unwrap();

        Node { node_ix }
    }

    pub fn outgoing_edges(&self, node: &Node) -> Vec<Edge> {
        self.collect_edges(node, Direction::Outgoing)
    }

    pub fn incoming_edges(&self, node: &Node) -> Vec<Edge> {
        self.collect_edges(node, Direction::Incoming)
    }

    /// Removes the edge from the graph but leaves the graph intact by keeping the empty
    /// edge in the graph by plucking the content of the edge, but not the edge itself.
    pub fn pluck_edge(&mut self, edge: Edge) -> QueryDependency {
        self.graph.edge_weight_mut(edge.edge_ix).unwrap().unset()
    }

    /// Removes the node from the graph but leaves the graph intact by keeping the empty
    /// node in the graph by plucking the content of the node, but not the node itself.
    pub fn pluck_node(&mut self, node: Node) -> Query {
        self.graph.node_weight_mut(node.node_ix).unwrap().unset()
    }

    /// Completely removes the edge from the graph, returning it's content.
    pub fn remove_edge(&mut self, edge: Edge) -> Option<QueryDependency> {
        self.graph.remove_edge(edge.edge_ix).unwrap().into_inner()
    }

    fn collect_edges(&self, node: &Node, direction: Direction) -> Vec<Edge> {
        let mut edges = self
            .graph
            .edges_directed(node.node_ix, direction)
            .map(|edge| Edge { edge_ix: edge.id() })
            .collect::<Vec<_>>();

        edges.sort();
        edges
    }

    fn stringify(&self) -> String {
        self.stringify_nodes(self.root_nodes()).join("\n\n")
    }

    fn stringify_nodes(&self, nodes: Vec<Node>) -> VecDeque<String> {
        let mut rendered_nodes = VecDeque::new();

        for node in nodes {
            let mut node_child_info = vec![];

            let children: Vec<Node> = self
                .outgoing_edges(&node)
                .iter()
                .map(|child_edge| {
                    let child_node = self.edge_target(child_edge);
                    node_child_info.push(format!(
                        "{}: {}",
                        child_node.id(),
                        self.edge_content(child_edge).unwrap()
                    ));

                    child_node
                })
                .collect();

            rendered_nodes.append(&mut self.stringify_nodes(children));

            rendered_nodes.prepend(format!(
                "Node {}: {}\n  {}",
                node.id(),
                self.node_content(&node).unwrap(),
                node_child_info.join("  \n")
            ));
        }

        rendered_nodes
    }
}
