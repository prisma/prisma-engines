///! Query graph abstraction for simple high-level query representation
///! and manipulation.
mod guard;

use connector::*;
use guard::*;
use petgraph::{graph::*, visit::EdgeRef, *};
use prisma_models::{PrismaValue, RelationFieldRef};
use std::{borrow::Borrow, cell::RefCell, ops::Deref};

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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    pub node_ix: NodeIndex,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    edge_ix: EdgeIndex,
}

// todo Read / Write distinction is only really important for ordering in the interpreter...we should try to get rid of that.
// todo Can we get rid of the relation field dependency?
/// Stored on the edges of the QueryGraph, a QueryDependency contains information on how children behave
/// relative to their parent during execution, for example requiring additional information from the parent to be able to execute.
pub enum QueryDependency {
    Write(RelationFieldRef, DependencyType<WriteQuery>),
    Read(DependencyType<ReadQuery>),
}

pub enum DependencyType<T> {
    /// Simple dependency indicating order of execution.
    ExecutionOrder,

    /// Performs a transformation on a query type T based on the parent ID (PrismaValue)
    ParentId(Box<dyn FnOnce(T, PrismaValue) -> T>),

    /// Expresses a conditional dependency that decides whether or not the child node
    /// is included in the execution.
    /// Currently, the evaluation function receives the parent ID as PrismaValue if it exists,
    /// None otherwise.
    Conditional(Box<dyn FnOnce(Option<PrismaValue>) -> bool>),
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
        let edge_ix = self
            .graph
            .add_edge(from.node_ix, to.node_ix, Guard::new(content));

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
        let mut edges = self
            .graph
            .edges_directed(node.node_ix, Direction::Outgoing)
            .map(|edge| Edge { edge_ix: edge.id() })
            .collect::<Vec<_>>();

        edges.sort();
        edges
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

    /// Current way to fix inconsistencies in the graph.
    // Todo This transformation could be encoded in the WriteQueryBuilder, making it possible to remove the relation field
    // on the graph edge.
    pub fn transform(mut self) -> Self {
        let graph = &mut self.graph;
        let candidates: Vec<EdgeIndex> = graph
            .raw_edges()
            .into_iter()
            .filter_map(|edge| {
                let parent = graph.node_weight(edge.source()).unwrap().borrow().unwrap();
                let child = graph.node_weight(edge.target()).unwrap().borrow().unwrap();
                let edge_index = graph.find_edge(edge.source(), edge.target()).unwrap();

                match (parent, child) {
                    (
                        Query::Write(WriteQuery::Root(RootWriteQuery::CreateRecord(_))),
                        Query::Write(WriteQuery::Root(RootWriteQuery::CreateRecord(_))),
                    ) => {
                        let relation_field: &RelationFieldRef = match edge.weight.borrow().unwrap() {
                            QueryDependency::Write(ref rf, _) => rf,
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
            let (parent, child) = graph.edge_endpoints(edge_index).unwrap();
            let edge = graph.remove_edge(edge_index).unwrap().unset();

            // Warning: This assumes that the DependencyType is also flippable.
            if let QueryDependency::Write(rf, op) = edge {
                graph.add_edge(
                    child,
                    parent,
                    Guard::new(QueryDependency::Write(rf.related_field(), op)),
                );
            }
        });

        self
    }
}
