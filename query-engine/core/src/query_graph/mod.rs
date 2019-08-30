///! Query graph abstraction for simple high-level query representation
///! and manipulation.
mod builder;

use crate::OutputTypeRef;
use builder::*;
use connector::*;
use petgraph::{graph::*, visit::EdgeRef, *};
use prisma_models::RelationFieldRef;

/// Implementation detail of the query graph.
type InnerGraph = Graph<Query, EdgeContent>;

#[derive(Debug, Default)]
pub struct QueryGraph {
    graph: InnerGraph,
}

pub struct Node<'a> {
    pub(self) graph: &'a QueryGraph,
    pub(self) node_ix: NodeIndex,
}

impl<'a> Node<'a> {
    pub fn edges(&self, direction: EdgeDirection) -> Vec<Edge> {
        self.graph.edges_for(self, direction)
    }

    pub fn content(&self) -> Query {
        self.graph.node_content(&self)
    }
}

pub enum EdgeDirection {
    Outgoing,
    Incoming,
}

pub struct Edge<'a> {
    pub(self) graph: &'a QueryGraph,
    pub(self) edge_ix: EdgeIndex,
}

impl<'a> Edge<'a> {
    pub fn source(&self) -> Node<'a> {
        unimplemented!()
    }

    pub fn target(&self) -> Node<'a> {
        unimplemented!()
    }

    pub fn content(&self) -> EdgeContent {
        self.graph.edge_content(self)
    }
}

#[derive(Debug, Clone)]
pub enum EdgeContent {
    Write(RelationFieldRef),
    Read,
}

impl From<Query> for QueryGraph {
    fn from(q: Query) -> Self {
        QueryGraphBuilder::build(q)
    }
}

impl<'a> QueryGraph {
    pub fn root_nodes(&'a self) -> Vec<Node<'a>> {
        self.graph
            .node_indices()
            .filter_map(|ix| {
                if let Some(_) = self.graph.edges_directed(ix, Direction::Incoming).next() {
                    None
                } else {
                    Some(ix)
                }
            })
            .map(|node_ix: NodeIndex| Node { graph: &self, node_ix })
            .collect()
    }

    pub fn node_content(&self, node: &Node) -> Query {
        self.graph.node_weight(node.node_ix).unwrap().clone()
    }

    pub fn edge_content(&self, edge: &Edge) -> EdgeContent {
        self.graph.edge_weight(edge.edge_ix).unwrap().clone()
    }

    pub fn edge_source(&'a self, edge: &Edge) -> Node<'a> {
        let (node_ix, _) = self.graph.edge_endpoints(edge.edge_ix).unwrap();

        Node { node_ix, graph: self }
    }

    pub fn edge_target(&'a self, edge: &Edge) -> Node<'a> {
        let (_, node_ix) = self.graph.edge_endpoints(edge.edge_ix).unwrap();

        Node { node_ix, graph: self }
    }

    pub fn edges_for(&'a self, node: &Node, direction: EdgeDirection) -> Vec<Edge<'a>> {
        let direction = match direction {
            EdgeDirection::Outgoing => Direction::Outgoing,
            EdgeDirection::Incoming => Direction::Incoming,
        };

        self.graph
            .edges_directed(node.node_ix, direction)
            .map(|edge| Edge {
                graph: self,
                edge_ix: edge.id(),
            })
            .collect()
    }
}
