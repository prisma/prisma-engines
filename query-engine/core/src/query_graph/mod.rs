///! Query graph abstraction for simple high-level query representation
///! and manipulation.
mod builder;

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
    pub fn new() -> Self {
        Self {
            graph: InnerGraph::new(),
        }
    }

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

    pub fn create_node(&'a mut self, query: Query) -> Node<'a> {
        let node_ix = self.graph.add_node(query);

        Node { node_ix, graph: self }
    }

    pub fn create_edge(&'a mut self, from: &Node<'a>, to: &Node<'a>, content: EdgeContent) -> Edge<'a> {
        let edge_ix = self.graph.add_edge(from.node_ix, to.node_ix, content);

        Edge { graph: self, edge_ix }
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

    /// Current way to fix inconsistencies in the graph.
    pub fn transform(mut self) -> Self {
        let candidates: Vec<EdgeIndex> = self
            .graph
            .raw_edges()
            .into_iter()
            .filter_map(|edge| {
                let parent = self.graph.node_weight(edge.source()).unwrap();
                let child = self.graph.node_weight(edge.target()).unwrap();
                let edge_index = self.graph.find_edge(edge.source(), edge.target()).unwrap();

                match (parent, child) {
                    (
                        Query::Write(WriteQuery::Root(RootWriteQuery::CreateRecord(_))),
                        Query::Write(WriteQuery::Root(RootWriteQuery::CreateRecord(_))),
                    ) => {
                        let relation_field: &RelationFieldRef = match &edge.weight {
                            EdgeContent::Write(rf) => rf,
                            _ => unreachable!(),
                        };

                        if relation_field.relation_is_inlined_in_parent() {
                            Some(edge_index)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect();

        candidates.into_iter().for_each(|edge_index| {
            let (parent, child) = self.graph.edge_endpoints(edge_index).unwrap();
            let edge = self.graph.remove_edge(edge_index).unwrap();

            if let EdgeContent::Write(rf) = edge {
                self.graph
                    .add_edge(child, parent, EdgeContent::Write(rf.related_field()));
            }
        });

        self
    }
}
