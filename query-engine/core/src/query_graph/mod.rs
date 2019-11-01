//! Query graph abstraction for simple high-level query representation and manipulation.
//! Wraps Petgraph crate graph.
mod error;
mod formatters;
mod guard;
mod invariance_rules;
mod transformers;

#[cfg(test)]
mod tests;

pub use error::*;
pub use formatters::*;
pub use transformers::*;

use crate::{Query, QueryGraphBuilderResult};
use guard::*;
use invariance_rules::*;
use petgraph::{graph::*, visit::EdgeRef as PEdgeRef, *};
use prisma_models::PrismaValue;
use std::borrow::Borrow;

pub type QueryGraphResult<T> = std::result::Result<T, QueryGraphError>;

pub enum Node {
    Query(Query),
    Flow(Flow),
}

impl From<Query> for Node {
    fn from(q: Query) -> Node {
        Node::Query(q)
    }
}

impl From<Flow> for Node {
    fn from(f: Flow) -> Node {
        Node::Flow(f)
    }
}

pub enum Flow {
    /// Expresses a conditional control flow in the graph.
    /// Possible outgoing edges are `then` and `else`, each at most once, with `then` required to be present.
    If(Box<dyn FnOnce() -> bool + Send + Sync + 'static>),

    /// Empty node, will return empty result on interpretation.
    /// Useful for checks that are only supposed to fail and noop on success.
    Empty,
}

impl Flow {
    pub fn default_if() -> Self {
        Self::If(Box::new(|| true))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct NodeRef {
    node_ix: NodeIndex,
}

impl NodeRef {
    /// Returns the unique identifier of the Node.
    pub fn id(&self) -> String {
        self.node_ix.index().to_string()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeRef {
    edge_ix: EdgeIndex,
}

impl EdgeRef {
    /// Returns the unique identifier of the Edge.
    pub fn id(&self) -> String {
        self.edge_ix.index().to_string()
    }
}

pub type ParentIdsFn = Box<dyn FnOnce(Node, Vec<PrismaValue>) -> QueryGraphBuilderResult<Node> + Send + Sync + 'static>;

/// Stored on the edges of the QueryGraph, a QueryGraphDependency contains information on how children are connected to their parents,
/// expressing for example the need for additional information from the parent to be able to execute at runtime.
pub enum QueryGraphDependency {
    /// Simple dependency indicating order of execution. Effectively a NOOP for now.
    ExecutionOrder,

    /// Performs a transformation on a node based on the IDs of the parent result (as PrismaValues).
    /// The result is typed to the builder result as the construction of the closures takes place in that module,
    /// and it avoids ugly hacks to combine the error types.
    ParentIds(ParentIdsFn),

    /// Only valid in the context of a `If` control flow node.
    Then,

    /// Only valid in the context of a `If` control flow node.
    Else,
}

/// A graph representing an abstract view of queries and their execution dependencies.
///
/// Graph invariants (TODO put checks into the code?):
/// - Directed, acyclic.
///
/// - Node IDs are unique and stable.
///
/// - The graph may have multiple result nodes, and multiple paths in the graph may point to result nodes, but only one result is serialized.
///   Note: The exact rules determining the final result are subject of the graph translation.
///
/// - Currently, Nodes are allowed to have multiple parents, but the following invariant applies: They may only refer to their parent and / or one of its ancestors.
///   Note: This rule guarantees that the dependent ancestor node result is always in scope for fulfillment of dependencies.
///
/// - Following the above, sibling dependencies are disallowed as well.
///
/// - Edges are ordered.
///   Node: Their evaluation is performed from low to high ordering, unless other rules require reshuffling the edges during translation.
#[derive(Default)]
pub struct QueryGraph {
    graph: InnerGraph,

    /// Designates the nodes that are returning the result of the entire QueryGraph.
    /// If no nodes are set, the interpretation will take the result of the
    /// last statement derived from the graph.
    result_nodes: Vec<NodeIndex>,

    /// Pairs of nodes marked for parent child swap.
    /// The first `NodeRef` of the tuple is the parent, the second one the child.
    /// The child will become the parents new parent when the graph is finalized.
    /// More docs can be found on `swap_marked`.
    marked_node_pairs: Vec<(NodeRef, NodeRef)>,

    finalized: bool,
}

/// Implementation detail of the QueryGraph.
type InnerGraph = Graph<Guard<Node>, Guard<QueryGraphDependency>>;

impl QueryGraph {
    pub fn new() -> Self {
        Self {
            graph: InnerGraph::new(),
            ..Default::default()
        }
    }

    pub fn finalize(&mut self) -> QueryGraphResult<()> {
        if !self.finalized {
            self.swap_marked()?;
            self.finalized = true;
        }

        self.validate()?;

        Ok(())
    }

    pub fn validate(&self) -> QueryGraphResult<()> {
        after_graph_completion(self)
    }

    /// Adds a result node to the graph.
    pub fn add_result_node(&mut self, node: &NodeRef) {
        self.result_nodes.push(node.node_ix.clone());
    }

    /// Checks if the given node is marked as one of the result nodes in the graph.
    pub fn is_result_node(&self, node: &NodeRef) -> bool {
        self.result_nodes
            .iter()
            .find(|rn| rn.index() == node.node_ix.index())
            .is_some()
    }

    /// Checks if the subgraph starting at the given node contains the node designated as the overall result.
    pub fn subgraph_contains_result(&self, node: &NodeRef) -> bool {
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

    /// Returns all root nodes of the graph.
    /// A root node is defined by having no incoming edges.
    pub fn root_nodes(&self) -> Vec<NodeRef> {
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
            .map(|node_ix: NodeIndex| NodeRef { node_ix })
            .collect()
    }

    /// Creates a node with content `t` and adds it to the graph.
    /// Returns a `NodeRef` to the newly added node.
    pub fn create_node<T>(&mut self, t: T) -> NodeRef
    where
        T: Into<Node>,
    {
        let node_ix = self.graph.add_node(Guard::new(t.into()));

        NodeRef { node_ix }
    }

    /// Creates an edge with given `content`, originating from node `from` and pointing to node `to`.
    /// Checks are run after edge creation to ensure validity of the query graph.
    /// Returns an `EdgeRef` to the newly added edge.
    /// Todo currently panics, change interface to result type.
    pub fn create_edge(
        &mut self,
        from: &NodeRef,
        to: &NodeRef,
        content: QueryGraphDependency,
    ) -> QueryGraphResult<EdgeRef> {
        let edge_ix = self.graph.add_edge(from.node_ix, to.node_ix, Guard::new(content));
        let edge = EdgeRef { edge_ix };

        after_edge_creation(self, &edge).map(|_| edge)
    }

    /// Returns a reference to the content of `node`, if the content is still present.
    pub fn node_content(&self, node: &NodeRef) -> Option<&Node> {
        self.graph.node_weight(node.node_ix).unwrap().borrow()
    }

    /// Returns a reference to the content of `edge`, if the content is still present.
    pub fn edge_content(&self, edge: &EdgeRef) -> Option<&QueryGraphDependency> {
        self.graph.edge_weight(edge.edge_ix).unwrap().borrow()
    }

    /// Returns the node from where `edge` originates (e.g. source).
    pub fn edge_source(&self, edge: &EdgeRef) -> NodeRef {
        let (node_ix, _) = self.graph.edge_endpoints(edge.edge_ix).unwrap();
        NodeRef { node_ix }
    }

    /// Returns the node to which `edge` points (e.g. target).
    pub fn edge_target(&self, edge: &EdgeRef) -> NodeRef {
        let (_, node_ix) = self.graph.edge_endpoints(edge.edge_ix).unwrap();
        NodeRef { node_ix }
    }

    /// Returns all edges originating from= `node` (e.g. outgoing edges).
    pub fn outgoing_edges(&self, node: &NodeRef) -> Vec<EdgeRef> {
        self.collect_edges(node, Direction::Outgoing)
    }

    /// Returns all edges pointing to `node` (e.g. incoming edges).
    pub fn incoming_edges(&self, node: &NodeRef) -> Vec<EdgeRef> {
        self.collect_edges(node, Direction::Incoming)
    }

    /// Removes the edge from the graph but leaves the graph intact by keeping the empty
    /// edge in the graph by plucking the content of the edge, but not the edge itself.
    pub fn pluck_edge(&mut self, edge: &EdgeRef) -> QueryGraphDependency {
        self.graph.edge_weight_mut(edge.edge_ix).unwrap().unset()
    }

    /// Removes the node from the graph but leaves the graph intact by keeping the empty
    /// node in the graph by plucking the content of the node, but not the node itself.
    pub fn pluck_node(&mut self, node: &NodeRef) -> Node {
        self.graph.node_weight_mut(node.node_ix).unwrap().unset()
    }

    /// Completely removes the edge from the graph, returning it's content.
    /// This operation is destructive on the underlying graph and invalidates
    pub fn remove_edge(&mut self, edge: EdgeRef) -> Option<QueryGraphDependency> {
        self.graph.remove_edge(edge.edge_ix).unwrap().into_inner()
    }

    /// Checks if `child` is a direct child of `parent`.
    ///
    /// Criteria for a direct child:
    /// - Every node that only has `parent` as their parent.
    /// - In case of multiple parents, has `parent` as their parent and _all_ other
    ///   parents are strict ancestors of `parent`, meaning they are "higher up" in the graph.
    pub fn is_direct_child(&self, parent: &NodeRef, child: &NodeRef) -> bool {
        self.incoming_edges(child).into_iter().all(|edge| {
            let ancestor = self.edge_source(&edge);

            if &ancestor != parent {
                self.is_ancestor(&ancestor, parent)
            } else {
                true
            }
        })
    }

    /// Returns a list of child nodes, together with their child edge for the given `node`.
    /// The list contains all children reachable by outgoing edges of `node`.
    pub fn child_pairs(&self, node: &NodeRef) -> Vec<(EdgeRef, NodeRef)> {
        self.outgoing_edges(node)
            .into_iter()
            .map(|edge| {
                let target = self.edge_target(&edge);
                (edge, target)
            })
            .collect()
    }

    /// Returns all direct child pairs of `node`.
    /// See `is_direct_child` for exact definition of what a direct child encompasses.
    pub fn direct_child_pairs(&self, node: &NodeRef) -> Vec<(EdgeRef, NodeRef)> {
        self.outgoing_edges(node)
            .into_iter()
            .filter_map(|edge| {
                let child_node = self.edge_target(&edge);

                if self.is_direct_child(node, &child_node) {
                    Some((edge, child_node))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Resolves and adds all source `NodeRef`s to the respective `EdgeRef`.
    pub fn zip_source_nodes(&self, edges: Vec<EdgeRef>) -> Vec<(EdgeRef, NodeRef)> {
        edges
            .into_iter()
            .map(|edge| {
                let target = self.edge_source(&edge);
                (edge, target)
            })
            .collect()
    }

    /// Checks if `ancestor` is in any of the ancestor nodes of `successor_node`.
    /// Determined by trying to reach `successor_node` from `ancestor`.
    pub fn is_ancestor(&self, ancestor: &NodeRef, successor_node: &NodeRef) -> bool {
        self.child_pairs(ancestor)
            .into_iter()
            .find(|(_, child_node)| child_node == successor_node || self.is_ancestor(&child_node, &successor_node))
            .is_some()
    }

    /// Internal utility function to collect all edges of defined direction directed to, or originating from, `node`.
    fn collect_edges(&self, node: &NodeRef, direction: Direction) -> Vec<EdgeRef> {
        let mut edges = self
            .graph
            .edges_directed(node.node_ix, direction)
            .map(|edge| EdgeRef { edge_ix: edge.id() })
            .collect::<Vec<_>>();

        edges.sort();
        edges
    }

    /// Marks a node pair for swapping.
    pub fn mark_nodes(&mut self, parent_node: &NodeRef, child_node: &NodeRef) {
        self.marked_node_pairs.push((parent_node.clone(), child_node.clone()));
    }

    /// Swaps all marked parent-child pairs.
    ///
    /// With this function, given a tuple of `(parent, child)`, `child` will be a parent node of `parent` after the swap has been performed.
    ///
    /// This operation preserves all edges from the parents of `parent` to the node, while inserting new edges from all parents of
    /// `parent` to `child`, effectively "pushing the child in the middle" of `parent` and it's parents. The new edges are only expressing
    /// exection order, and no node transformation like `ParentIds`.
    ///
    /// Any edge existing between `parent` and `child` will change direction and will point from `child` to `parent` instead.
    ///
    /// **Important exception**: If a parent node is a `Flow` node, we need to completely remove the edge to the flow node and rewire it to the child.
    ///
    /// ## Example transformation
    /// Given the marked pairs `[(A, B), (B, C), (B, D)]` and a graph (depicting the state before the transformation):
    /// ```text
    ///      ┌───┐
    ///      │ P │
    ///      └───┘
    ///        │
    ///        ▼
    ///      ┌───┐
    ///      │ A │
    ///      └───┘
    ///        │
    ///        ▼
    ///      ┌───┐
    ///   ┌──│ B │──┐
    ///   │  └───┘  │
    ///   │         │
    ///   ▼         ▼
    /// ┌───┐     ┌───┐
    /// │ C │     │ D │
    /// └───┘     └───┘
    /// ```
    ///
    /// The marked pairs express that the operations performed by the contained parents depend on the child operation,
    /// hence making it necessary to execute the child operation first.
    ///
    /// Applying the transformations step by step will change the graph as following:
    /// (original edges marked with an ID to show how they're preserved)
    /// ```text
    ///      ┌───┐                 ┌───┐                                 ┌───┐                         ┌───┐
    ///      │ P │                 │ P │────────────────┐             ┌──│ P │──────────┐           ┌──│ P │────────┐
    ///      └───┘                 └───┘               1│             │  └───┘         1│           │  └───┘       1│
    ///       1│                     │                  │            5│   6│            │          5│   6│          │
    ///        ▼                     │                  │             │    ▼            │           │    ▼          │
    ///      ┌───┐                   │                  │             │  ┌───┐          │           │  ┌───┐        │
    ///      │ A │                  5│                  │             │  │ C │          │           │  │ C │───┐    │
    ///      └───┘     ═(A, B)═▶     │                  │  ═(B, C)═▶  │  └───┘          │ ═(B, D)═▶ │  └───┘  7│    │
    ///       2│                     │                  │             │   3│            │           │   3│     │    │
    ///        ▼                     ▼                  │             │    ▼            │           │    │     ▼    │
    ///      ┌───┐                 ┌───┐                │             │  ┌───┐          │           │    │   ┌───┐  │
    ///   ┌──│ B │──┐           ┌──│ B │──┬────────┐    │             └─▶│ B │─────┐    │           │    │   │ D │  │
    ///  3│  └───┘ 4│          3│  └───┘ 4│       2│    │                └───┘     │    │           │    │   └───┘  │
    ///   │         │           │         │        │    │                 4│      2│    │           │    │    4│    │
    ///   ▼         ▼           ▼         ▼        ▼    │                  ▼       ▼    │           │    ▼     │    │
    /// ┌───┐     ┌───┐       ┌───┐     ┌───┐    ┌───┐  │                ┌───┐   ┌───┐  │           │  ┌───┐   │    │
    /// │ C │     │ D │       │ C │     │ D │    │ A │◀─┘                │ D │   │ A │◀─┘           └─▶│ B │◀──┘    │
    /// └───┘     └───┘       └───┘     └───┘    └───┘                   └───┘   └───┘                 └───┘        │
    ///                                                                                                  │          │
    ///                                                                                                  │          │
    ///                                                                                                 2│   ┌───┐  │
    ///                                                                                                  └──▶│ A │◀─┘
    ///                                                                                                      └───┘
    /// ```
    /// todo put if flow exception illustration here.
    fn swap_marked(&mut self) -> QueryGraphResult<()> {
        if self.marked_node_pairs.len() > 0 {
            trace!("[Graph][Swap] Before shape: {}", self);
        }

        let mut marked = std::mem::replace(&mut self.marked_node_pairs, vec![]);
        marked.reverse(); // Todo: Marked operation order is currently breaking if done bottom-up. Investigate how to fix it.

        for (parent_node, child_node) in marked {
            // All parents of `parent_node` are becoming a parent of `child_node` as well, except flow nodes.
            let parent_edges = self.incoming_edges(&parent_node);
            for parent_edge in parent_edges {
                let parent_of_parent_node = self.edge_source(&parent_edge);

                match self
                    .node_content(&parent_of_parent_node)
                    .expect("Expected marked nodes to be non-empty.")
                {
                    Node::Flow(_) => {
                        let content = self
                            .remove_edge(parent_edge)
                            .expect("Expected edges between marked nodes to be non-empty.");
                        self.create_edge(&parent_of_parent_node, &child_node, content)?;
                    }

                    _ => {
                        trace!(
                            "[Graph][Swap] Connecting parent of parent {} with child {}",
                            parent_of_parent_node.id(),
                            child_node.id()
                        );

                        self.create_edge(
                            &parent_of_parent_node,
                            &child_node,
                            QueryGraphDependency::ExecutionOrder,
                        )?;
                    }
                }
            }

            // Find existing edge between parent and child. Can only be one at most.
            let existing_edge = self
                .graph
                .find_edge(parent_node.node_ix, child_node.node_ix)
                .map(|edge_ix| EdgeRef { edge_ix });

            // Remove edge and reinsert edge in reverse.
            if let Some(edge) = existing_edge {
                let content = self.remove_edge(edge).unwrap();
                self.create_edge(&child_node, &parent_node, content)?;
            }
        }

        Ok(())
    }
}
