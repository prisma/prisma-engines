mod error;
mod formatters;
mod guard;
mod transformers;

use std::fmt;

pub use error::*;
use psl::datamodel_connector::{ConnectorCapabilities, ConnectorCapability};
use serde::Serialize;
use smallvec::{smallvec, SmallVec};

use crate::{
    interpreter::ExpressionResult, FilteredQuery, ManyRecordsQuery, Query, QueryGraphBuilderError,
    QueryGraphBuilderResult, QueryOptions, ReadQuery,
};
use guard::*;
use itertools::Itertools;
use petgraph::{
    graph::*,
    visit::{EdgeRef as PEdgeRef, NodeIndexable},
    *,
};
use query_structure::{FieldSelection, IntoFilter, QueryArguments, SelectionResult};

pub type QueryGraphResult<T> = std::result::Result<T, QueryGraphError>;

#[allow(clippy::large_enum_variant)]
pub enum Node {
    /// Nodes representing actual queries to the underlying connector.
    Query(Query),

    /// Flow control nodes.
    Flow(Flow),

    // Todo this strongly indicates that the query graph has to change, probably towards a true AST for the interpretation,
    // instead of this unsatisfying in-between of high-level abstraction over the incoming query and concrete interpreter actions.
    /// A general computation to perform. As opposed to `Query`, this doesn't invoke the connector.
    Computation(Computation),

    /// Empty node.
    Empty,
}

impl Node {
    pub(crate) fn as_query(&self) -> Option<&Query> {
        if let Self::Query(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub(crate) fn as_query_mut(&mut self) -> Option<&mut Query> {
        if let Self::Query(v) = self {
            Some(v)
        } else {
            None
        }
    }
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
    If { rule: DataRule, data: Vec<SelectionResult> },

    /// Returns a fixed set of results at runtime.
    Return(Vec<SelectionResult>),
}

impl Flow {
    pub fn if_non_empty() -> Self {
        Self::If {
            rule: DataRule::RowCountNeq(0),
            data: Vec::new(),
        }
    }

    pub fn if_false() -> Self {
        Self::If {
            rule: DataRule::Never,
            data: Vec::new(),
        }
    }
}

// Current limitation: We need to narrow it down to ID diffs for Hash and EQ.
pub enum Computation {
    DiffLeftToRight(DiffNode),
    DiffRightToLeft(DiffNode),
}

impl Computation {
    pub fn empty_diff_left_to_right() -> Self {
        Self::DiffLeftToRight(DiffNode::default())
    }

    pub fn empty_diff_right_to_left() -> Self {
        Self::DiffRightToLeft(DiffNode::default())
    }
}

#[derive(Default)]
pub struct DiffNode {
    pub left: Vec<SelectionResult>,
    pub right: Vec<SelectionResult>,
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

pub(crate) type ProjectedDataDependencyFn =
    Box<dyn FnOnce(Node, Vec<SelectionResult>) -> QueryGraphBuilderResult<Node> + Send + Sync + 'static>;

pub(crate) type DataDependencyFn =
    Box<dyn FnOnce(Node, &ExpressionResult) -> QueryGraphBuilderResult<Node> + Send + Sync + 'static>;

/// Stored on the edges of the QueryGraph, a QueryGraphDependency contains information on how children are connected to their parents,
/// expressing for example the need for additional information from the parent to be able to execute at runtime.
pub enum QueryGraphDependency {
    /// Simple dependency indicating order of execution. Effectively an ordering and reachability tool for now.
    ExecutionOrder,

    /// Performs a transformation on the target node based on the source node result..
    DataDependency(DataDependencyFn),

    /// More specialized version of `DataDependency` with more guarantees and side effects.
    ///
    /// Performs a transformation on the target node based on the requested selection on the source result (represented as a single merged `FieldSelection`).
    /// Assumes that the source result can be converted into the requested selection, else a runtime error will occur.
    /// The `FieldSelection` is used to determine the set of values to extract from the source result.
    ///
    /// Important note: As opposed to `DataDependency`, this dependency guarantees that if the closure is called, the source result contains at least the requested selection.
    /// To achieve that, the query graph is post-processed in the `finalize` and reloads are injected at points where a selection is not fulfilled.
    /// See `insert_reloads` for more information.
    ProjectedDataDependency(FieldSelection, ProjectedDataDependencyFn, Option<DataExpectation>), // [Composites] todo rename

    ProjectedDataSinkDependency(FieldSelection, DataSink, Option<DataExpectation>),

    /// Only valid in the context of a `If` control flow node.
    Then,

    /// Only valid in the context of a `If` control flow node.
    Else,
}

#[derive(Debug)]
pub enum DataSink {
    AllRows(&'static dyn NodeInputField<Vec<SelectionResult>>),
    SingleRow(&'static dyn NodeInputField<SelectionResult>),
    SingleRowArray(&'static dyn NodeInputField<Vec<SelectionResult>>),
}

pub trait NodeInputField<R>: Send + Sync + fmt::Debug {
    fn node_input_field<'a>(&self, node: &'a mut Node) -> &'a mut R;
}

/// An expectation for a data dependency.
pub struct DataExpectation {
    rules: SmallVec<[DataRule; 1]>,
    error: Box<dyn DataDependencyError>,
}

impl DataExpectation {
    pub fn non_empty_rows(error: impl DataDependencyError + 'static) -> Self {
        Self {
            rules: smallvec![DataRule::RowCountNeq(0)],
            error: Box::new(error),
        }
    }

    pub fn empty_rows(error: impl DataDependencyError + 'static) -> Self {
        Self {
            rules: smallvec![DataRule::RowCountEq(0)],
            error: Box::new(error),
        }
    }

    pub fn exact_row_count(expected: usize, error: impl DataDependencyError + 'static) -> Self {
        Self {
            rules: smallvec![DataRule::RowCountEq(expected)],
            error: Box::new(error),
        }
    }

    pub fn rules(&self) -> &[DataRule] {
        &self.rules
    }

    pub fn error(&self) -> &dyn DataDependencyError {
        &*self.error
    }

    pub fn check(&self, results: &[SelectionResult]) -> Result<(), QueryGraphBuilderError> {
        for rule in &self.rules {
            if !rule.matches_data(results) {
                return Err(self.error.to_runtime_error(results));
            }
        }
        Ok(())
    }
}

/// A rule a data dependency needs to fulfill to be considered valid.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "args", rename_all = "camelCase")]
pub enum DataRule {
    /// Expect the data dependency to contain an exact number of rows.
    RowCountEq(usize),
    /// Expect the data dependency to contain a number of rows that is not equal to the given value.
    RowCountNeq(usize),
    /// Expect the edge to not be taken and never match any data.
    Never,
}

impl DataRule {
    pub fn matches_data(&self, results: &[SelectionResult]) -> bool {
        match self {
            Self::RowCountEq(expected) => results.len() == *expected,
            Self::RowCountNeq(expected) => results.len() != *expected,
            Self::Never => false,
        }
    }
}

impl fmt::Display for DataRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RowCountEq(expected) => write!(f, "rowCountEq {expected}"),
            Self::RowCountNeq(expected) => write!(f, "rowCountNeq {expected}"),
            Self::Never => write!(f, "never"),
        }
    }
}

/// An error that can occur during data dependency validation.
pub trait DataDependencyError: Send + Sync {
    /// A unique identifier for the error.
    fn id(&self) -> &'static str;
    /// Converts the error into a runtime query engine error.
    fn to_runtime_error(&self, results: &[SelectionResult]) -> QueryGraphBuilderError;
    /// Context with additional information used to provide more context for the error.
    fn context(&self) -> serde_json::Value;
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

    /// For now a simple marker if the query graph needs to be run inside a
    /// transaction. Should happen if any of the queries is writing data.
    needs_transaction: bool,

    /// Already visited nodes.
    /// Nodes are visited during query graph processing.
    /// Influences traversal rules and how child nodes are treated.
    visited: Vec<NodeIndex>,
}

impl fmt::Debug for QueryGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryGraph")
            .field("graph", &"InnerGraph")
            .field("result_nodes", &self.result_nodes)
            .field("marked_node_pairs", &self.marked_node_pairs)
            .field("finalized", &self.finalized)
            .field("needs_transaction", &self.needs_transaction)
            .field("visited", &self.visited)
            .finish()
    }
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

    pub(crate) fn root<F>(f: F) -> QueryGraphBuilderResult<QueryGraph>
    where
        F: FnOnce(&mut QueryGraph) -> QueryGraphBuilderResult<()>,
    {
        let mut graph = QueryGraph::new();
        f(&mut graph)?;
        Ok(graph)
    }

    pub fn finalize(&mut self, capabilities: ConnectorCapabilities) -> QueryGraphResult<()> {
        if !self.finalized {
            self.swap_marked()?;
            self.ensure_return_nodes_have_parent_dependency()?;
            self.normalize_data_dependencies(capabilities)?;
            self.insert_reloads()?;
            self.normalize_if_nodes()?;
            self.finalized = true;
        }

        Ok(())
    }

    pub fn result_nodes(&self) -> impl Iterator<Item = NodeRef> + '_ {
        self.result_nodes.iter().map(|node_ix| NodeRef { node_ix: *node_ix })
    }

    /// Adds a result node to the graph.
    pub fn add_result_node(&mut self, node: &NodeRef) {
        self.result_nodes.push(node.node_ix);
    }

    pub fn mark_visited(&mut self, node: &NodeRef) {
        if !self.visited.contains(&node.node_ix) {
            trace!("Visited: {}", node.id());
            self.visited.push(node.node_ix);
        }
    }

    /// Checks if the given node is marked as one of the result nodes in the graph.
    pub fn is_result_node(&self, node: &NodeRef) -> bool {
        self.result_nodes.iter().any(|rn| rn.index() == node.node_ix.index())
    }

    /// Checks if the subgraph starting at the given node contains the node designated as the overall result.
    pub fn subgraph_contains_result(&self, node: &NodeRef) -> bool {
        if self.is_result_node(node) {
            true
        } else {
            self.outgoing_edges(node).into_iter().any(|edge| {
                let child_node = self.edge_target(&edge);
                self.subgraph_contains_result(&child_node)
            })
        }
    }

    /// Returns all root nodes of the graph.
    /// A root node is defined by having no incoming edges.
    pub fn root_nodes(&self) -> impl Iterator<Item = NodeRef> + '_ {
        self.graph.node_indices().filter_map(|node_ix| {
            if self.graph.edges_directed(node_ix, Direction::Incoming).next().is_some() {
                None
            } else {
                Some(NodeRef { node_ix })
            }
        })
    }

    /// Creates a node with content `t` and adds it to the graph.
    /// Returns a `NodeRef` to the newly added node.
    pub(crate) fn create_node<T>(&mut self, t: T) -> NodeRef
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
    pub(crate) fn create_edge(
        &mut self,
        from: &NodeRef,
        to: &NodeRef,
        content: QueryGraphDependency,
    ) -> QueryGraphResult<EdgeRef> {
        let edge_ix = self.graph.add_edge(from.node_ix, to.node_ix, Guard::new(content));
        let edge = EdgeRef { edge_ix };

        Ok(edge)
    }

    /// Mark the query graph to need a transaction.
    pub(crate) fn flag_transactional(&mut self) {
        self.needs_transaction = true;
    }

    /// If true, the graph should be executed inside of a transaction.
    pub fn needs_transaction(&self) -> bool {
        self.needs_transaction
    }

    /// Returns a reference to the content of `node`, if the content is still present.
    pub fn node_content(&self, node: &NodeRef) -> Option<&Node> {
        self.graph.node_weight(node.node_ix).unwrap().borrow()
    }

    /// Returns a reference to the content of `node`, if the content is still present.
    pub(crate) fn node_content_mut(&mut self, node: &NodeRef) -> Option<&mut Node> {
        self.graph.node_weight_mut(node.node_ix).unwrap().borrow_mut()
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
    pub(crate) fn edge_target(&self, edge: &EdgeRef) -> NodeRef {
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
    /// Panics if the edge has been already been taken or plucked.
    pub fn pluck_edge(&mut self, edge: &EdgeRef) -> QueryGraphDependency {
        self.graph.edge_weight_mut(edge.edge_ix).unwrap().unset()
    }

    /// Removes the edge from the graph but leaves the graph intact by keeping the empty
    /// edge in the graph by taking the content of the edge, but not the edge itself.
    /// Returns `None` if the edge has been already taken or plucked.
    pub fn take_edge(&mut self, edge: &EdgeRef) -> Option<QueryGraphDependency> {
        self.graph.edge_weight_mut(edge.edge_ix).unwrap().take()
    }

    /// Removes the node from the graph but leaves the graph intact by keeping the empty
    /// node in the graph by plucking the content of the node, but not the node itself.
    pub fn pluck_node(&mut self, node: &NodeRef) -> Node {
        self.graph.node_weight_mut(node.node_ix).unwrap().unset()
    }

    /// Completely removes the edge from the graph, returning it's content.
    /// This operation is destructive on the underlying graph and invalidates references.
    pub(crate) fn remove_edge(&mut self, edge: EdgeRef) -> Option<QueryGraphDependency> {
        self.graph.remove_edge(edge.edge_ix).unwrap().into_inner()
    }

    /// Checks if `child` is a direct child of `parent`.
    ///
    /// Criteria for a direct child:
    /// - The edge has not been plucked or taken.
    /// - `parent` is this node's parent.
    /// - In case of multiple parents, all other parents have already been visited before.
    pub fn is_direct_child(&self, parent: &NodeRef, child: &NodeRef) -> bool {
        self.incoming_edges(child).into_iter().all(|edge| {
            let other_parent = self.edge_source(&edge);

            if self.edge_content(&edge).is_none() {
                return false;
            }

            &other_parent == parent || self.visited.contains(&other_parent.node_ix)
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
        self.marked_node_pairs.push((*parent_node, *child_node));
    }

    /// Swaps all marked parent-child pairs.
    ///
    /// With this function, given a tuple of `(parent, child)`, `child` will be a parent node of `parent` after the swap has been performed.
    ///
    /// This operation preserves all edges from the parents of `parent` to the node, while inserting new edges from all parents of
    /// `parent` to `child`, effectively "pushing the child in the middle" of `parent` and it's parents. The new edges are only expressing
    /// exection order, and no node transformation.
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
    /// (new edges created in a transformation step are marked with *)
    /// ```text
    ///         ┌───┐                 ┌───┐                                 ┌───┐                         ┌───┐
    ///         │ P │                 │ P │────────────────┐             ┌──│ P │──────────┐           ┌──│ P │────────┐
    ///         └───┘                 └───┘               1│             │  └───┘         1│           │  └───┘       1│
    ///          1│                     │                  │            5│    │ 6*         │          5│   6│          │
    ///           ▼                     │                  │             │    ▼            │           │    ▼          │
    ///         ┌───┐                   │                  │             │  ┌───┐          │           │  ┌───┐  7*    │
    ///         │ A │                 5*│                  │             │  │ C │          │           │  │ C │───┐    │
    ///         └───┘     ═(A, B)═▶     │                  │ ═(B, C)═▶   │  └───┘          │ ═(B, D)═▶ │  └───┘   │    │
    ///          2│                     │                  │             │   3│            │           │   3│     │    │
    ///           ▼                     ▼                  │             │    ▼            │           │    │     ▼    │
    ///         ┌───┐                 ┌───┐                │             │  ┌───┐          │           │    │   ┌───┐  │
    ///      ┌──│ B │──┐           ┌──│ B │──┬────────┐    │             └─▶│ B │─────┐    │           │    │   │ D │  │
    ///     3│  └───┘ 4│          3│  └───┘ 4│       2│    │                └───┘     │    │           │    │   └───┘  │
    ///      │         │           │         │        │    │                 4│      2│    │           │    │    4│    │
    ///      ▼         ▼           ▼         ▼        ▼    │                  ▼       ▼    │           │    ▼     │    │
    ///    ┌───┐     ┌───┐       ┌───┐     ┌───┐    ┌───┐  │                ┌───┐   ┌───┐  │           │  ┌───┐   │    │
    ///    │ C │     │ D │       │ C │     │ D │    │ A │◀─┘                │ D │   │ A │◀─┘           └─▶│ B │◀──┘    │
    ///    └───┘     └───┘       └───┘     └───┘    └───┘                   └───┘   └───┘                 └───┘        │
    ///                                                                                                     │          │
    ///                                                                                                     │          │
    ///                                                                                                    2│   ┌───┐  │
    ///                                                                                                     └──▶│ A │◀─┘
    ///                                                                                                         └───┘
    /// ```
    /// [DTODO] put if flow exception illustration here.
    fn swap_marked(&mut self) -> QueryGraphResult<()> {
        if !self.marked_node_pairs.is_empty() {
            trace!("[Graph][Swap] Before shape: {}", self);
        }

        let mut marked = std::mem::take(&mut self.marked_node_pairs);
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
                    // Exception rule: Only swap `Then` and `Else` edges.
                    Node::Flow(Flow::If { .. }) => {
                        if matches!(
                            self.edge_content(&parent_edge),
                            Some(QueryGraphDependency::Then) | Some(QueryGraphDependency::Else)
                        ) {
                            let content = self
                                .remove_edge(parent_edge)
                                .expect("Expected edges between marked nodes to be non-empty.");

                            self.create_edge(&parent_of_parent_node, &child_node, content)?;
                        }
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
                let content = self.pluck_edge(&edge);
                self.create_edge(&child_node, &parent_node, content)?;
                self.remove_edge(edge);
            }
        }

        Ok(())
    }

    /// Inserts ordering edges into the graph to prevent interdependency issues when rotating
    /// nodes for `if`-flow nodes.
    ///
    /// All sibling nodes of an if-node that are...
    /// - ... not an `if`-flow node themself
    /// - ... not already connected to the current `if`-flow node in any form (to prevent double edges)
    /// - ... not connected to another `if`-flow node with control flow edges (indirect sibling)
    ///
    /// will be ordered below the currently processed `if`-flow node in execution predence.
    ///
    /// ```text
    ///      ┌ ─ ─ ─ ─ ─ ─
    /// ┌ ─ ─    Parent   │─ ─ ─ ─ ┬ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┬ ─ ─ ─ ─
    ///      └ ─ ─ ─ ─ ─ ─                        │                       │
    /// │           │              │                             │
    ///             │                             │                       │
    /// │           │              │                             │
    ///             ▼              ▼              ▼              ▼        │
    /// │    ┌ ─ ─ ─ ─ ─ ─  ┌ ─ ─ ─ ─ ─ ─  ┌ ─ ─ ─ ─ ─ ─  ┌ ─ ─ ─ ─ ─ ─
    ///   ┌ ─      If     │    Sibling   │   Sibling If │   Sibling If │  │
    /// │    └ ─ ─ ─ ─ ─ ─  └ ─ ─ ─ ─ ─ ─  └ ─ ─ ─ ─ ─ ─  └ ─ ─ ─ ─ ─ ─
    ///   │         │              ▲                             │        │
    /// │           │              │
    ///   │         └────Inserted ─┘                       (Then / Else)  │
    /// │                Ordering
    ///   │                                                      ▼        │
    /// │    ┌ ─ ─ ─ ─ ─ ─                                ┌ ─ ─ ─ ─ ─ ─
    ///   │     Already   │                                  Indirect  │  │
    /// └ ──▶│ connected                                  │  sibling    ◀─
    ///         sibling   │                                ─ ─ ─ ─ ─ ─ ┘
    ///      └ ─ ─ ─ ─ ─ ─
    /// ```
    fn normalize_if_nodes(&mut self) -> QueryGraphResult<()> {
        for node_ix in self.graph.node_indices() {
            let node = NodeRef { node_ix };

            if let Node::Flow(Flow::If { .. }) = self.node_content(&node).unwrap() {
                let parents = self.incoming_edges(&node);

                for parent_edge in parents {
                    let parent = self.edge_source(&parent_edge);
                    let siblings = self.child_pairs(&parent);

                    for (_, sibling) in siblings {
                        let possible_edge = self.graph.find_edge(node.node_ix, sibling.node_ix);
                        let is_if_node_child = self.incoming_edges(&sibling).into_iter().any(|edge| {
                            let content = self.edge_content(&edge).unwrap();
                            matches!(content, QueryGraphDependency::Then | QueryGraphDependency::Else)
                        });

                        if sibling != node
                            && possible_edge.is_none()
                            && !is_if_node_child
                            && !matches!(self.node_content(&sibling).unwrap(), Node::Flow(_))
                        {
                            self.create_edge(&node, &sibling, QueryGraphDependency::ExecutionOrder)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Traverses the graph and ensures that return nodes have correct `ProjectedDataDependency`s on their incoming edges.
    ///
    /// Steps:
    /// - Collect & merge the outgoing edge dependencies into a single `FieldSelection`
    /// - Transform the incoming edge dependencies of the return nodes with the merged outgoing edge dependencies of the previous step
    ///
    /// This ensures that children nodes of return nodes have the proper data dependencies at their disposal.
    /// In case the parent nodes of return nodes do not have a field selection that fullfils the new dependency,
    /// a reload node will be inserted in between the parent and the return node by the `insert_reloads` method.
    fn ensure_return_nodes_have_parent_dependency(&mut self) -> QueryGraphResult<()> {
        let return_nodes: Vec<NodeRef> = self
            .graph
            .node_indices()
            .filter_map(|ix| {
                let node = NodeRef { node_ix: ix };

                match self.node_content(&node).unwrap() {
                    Node::Flow(Flow::Return(_)) => Some(node),
                    _ => None,
                }
            })
            .collect();

        for return_node in return_nodes {
            let out_edges = self.outgoing_edges(&return_node);
            let dependencies: Vec<FieldSelection> = out_edges
                .into_iter()
                .filter_map(|edge| match self.edge_content(&edge).unwrap() {
                    QueryGraphDependency::ProjectedDataDependency(ref requested_selection, _, _)
                    | QueryGraphDependency::ProjectedDataSinkDependency(ref requested_selection, _, _) => {
                        Some(requested_selection.clone())
                    }
                    _ => None,
                })
                .collect();
            let dependencies = FieldSelection::union(dependencies);

            // Assumption: We currently always have at most one single incoming ProjectedDataDependency edge
            // connected to return nodes. This will break if we ever have more.
            let in_edges = self.incoming_edges(&return_node);
            let incoming_dep_edge = in_edges.into_iter().find(|edge| {
                matches!(
                    self.edge_content(edge),
                    Some(
                        QueryGraphDependency::ProjectedDataDependency(_, _, _)
                            | QueryGraphDependency::ProjectedDataSinkDependency(_, _, _)
                    )
                )
            });

            if let Some(incoming_edge) = incoming_dep_edge {
                let source = self.edge_source(&incoming_edge);
                let target = self.edge_target(&incoming_edge);
                let content = self
                    .remove_edge(incoming_edge)
                    .expect("Expected edges between marked nodes to be non-empty.");

                match content {
                    QueryGraphDependency::ProjectedDataDependency(existing, transformer, expectation) => {
                        let merged_dependencies = dependencies.merge(existing);
                        self.create_edge(
                            &source,
                            &target,
                            QueryGraphDependency::ProjectedDataDependency(
                                merged_dependencies,
                                transformer,
                                expectation,
                            ),
                        )?;
                    }
                    QueryGraphDependency::ProjectedDataSinkDependency(existing, sink, expectation) => {
                        let merged_dependencies = dependencies.merge(existing);
                        self.create_edge(
                            &source,
                            &target,
                            QueryGraphDependency::ProjectedDataSinkDependency(merged_dependencies, sink, expectation),
                        )?;
                    }
                    _ => (),
                }
            }
        }

        Ok(())
    }

    /// Traverses the query graph and checks if reloads of nodes are necessary.
    /// Whether or not a node needs to be reloaded is determined based on the
    /// incoming `ProjectedDataDependency` edge transformers, as those hold the `FieldSelection`s
    /// all records of the source result need to contain in order to satisfy dependencies.
    ///
    /// If a node needs to be reloaded, ALL edges going out from the reloaded node need to be rewired, not
    /// only unsatified ones.
    ///
    /// ## Example
    /// Given a query graph, where 3 children require different set of fields ((A, B), (B, C), (A, D))
    /// to execute their dependent operations:
    /// ```text
    /// ┌ ─ ─ ─ ─ ─ ─
    ///     Parent   │─────────┬───────────────┐
    /// └ ─ ─ ─ ─ ─ ─          │               │
    ///        │               │               │
    ///     (A, B)          (B, C)           (A, D)
    ///        │               │               │
    ///        ▼               ▼               ▼
    /// ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─
    ///    Child A   │     Child B   │     Child C   │
    /// └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─
    /// ```
    /// However, `Parent` only returns `(A, B)`, for example, because that's the primary ID of the parent model
    /// and `Parent` is an operation that only returns IDs (e.g. update, updateMany).
    ///
    /// In order to satisfy children B and C, the graph is altered by this post-processing call:
    /// ```text
    /// ┌ ─ ─ ─ ─ ─ ─
    ///     Parent   │
    /// └ ─ ─ ─ ─ ─ ─
    ///        │
    ///     (A, B) (== Primary ID)
    ///        │
    ///        ▼
    /// ┌────────────┐
    /// │   Reload   │─────────┬───────────────┐
    /// └────────────┘         │               │
    ///        │               │               │
    ///     (A, B)          (B, C)           (A, D)
    ///        │               │               │
    ///        ▼               ▼               ▼
    /// ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─
    ///    Child A   │     Child B   │     Child C   │
    /// └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─
    /// ```
    ///
    /// The edges from `Parent` to all dependent children are removed from the graph and reinserted in order
    /// on the reload node.
    ///
    /// The `Reload` node is always a "find many" query.
    /// Unwraps are safe because we're operating on the unprocessed state of the graph (`Expressionista` changes that).
    fn insert_reloads(&mut self) -> QueryGraphResult<()> {
        let reloads = self.find_unsatisfied_dependencies();

        for (node, identifiers) in reloads {
            let query = self.node_content(&node).and_then(|node| node.as_query()).unwrap();

            trace!(
                "Query {:?} does not return requested selection {:?} and will be reloaded.",
                query,
                identifiers.prisma_names().collect::<Vec<_>>()
            );

            // Create reload node and connect it to the `node`.
            let model = query.model();
            let primary_model_id = model.primary_identifier();

            let read_query = ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
                name: "reload".into(),
                alias: None,
                model: model.clone(),
                args: QueryArguments::new(model),
                selected_fields: identifiers.merge(primary_model_id.clone()),
                nested: vec![],
                selection_order: vec![],
                options: QueryOptions::none(),
                relation_load_strategy: query_structure::RelationLoadStrategy::Query,
            });

            let reload_query = Query::Read(read_query);
            let reload_node = self.create_node(reload_query);

            self.create_edge(
                &node,
                &reload_node,
                QueryGraphDependency::ProjectedDataDependency(
                    primary_model_id,
                    Box::new(|mut reload_node, parent_result| {
                        if let Node::Query(Query::Read(ReadQuery::ManyRecordsQuery(ref mut mr))) = reload_node {
                            mr.set_filter(parent_result.filter());
                        }

                        Ok(reload_node)
                    }),
                    None,
                ),
            )?;

            // Remove all edges from node to children, reattach them to the reload node
            for edge in self.outgoing_edges(&node) {
                let target = self.edge_target(&edge);
                let content = self.remove_edge(edge).unwrap();

                self.create_edge(&reload_node, &target, content)?;
            }
        }

        Ok(())
    }

    /// Traverses the query graph and finds the nodes that need their selection set to be updated so that they fulfill the data dependencies of their children.
    /// We determine that based on incoming `ProjectedDataDependency` edge transformers, as those hold the `FieldSelection`s
    /// that all records of the source result need to contain in order to satisfy dependencies.
    ///
    /// ## Example
    /// Given a query graph, where 3 children require different set of fields ((A, B), (B, C), (A, D))
    /// to execute their dependent operations:
    /// ```text
    /// ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─
    ///     Parent (A, B)  │─────────┬───────────────┐
    /// └ ─ ─ ─ ─ ─ ─  ─ ─           │               │
    ///        │                     │               │
    ///     (A, B)                (B, C)           (A, D)
    ///        │                    │               │
    ///        ▼                    ▼               ▼
    /// ┌ ─ ─ ─ ─ ─ ─          ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─
    ///    Child A   │         |  Child B   │  |  Child C   │
    /// └ ─ ─ ─ ─ ─ ─          └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─
    /// ```
    /// However, `Parent` only returns `(A, B)`, for example, because that's the primary ID of the parent model.
    ///
    /// In order to satisfy children B and C, the graph is altered by this post-processing call:
    /// ```text
    /// ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
    ///     Parent (A, B, C, D)  │─────────┬───────────────┐
    /// └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─          │               │
    ///        │                           │               │
    ///     (A, B)                       (B, C)           (A, D)
    ///        │                           │               │
    ///        ▼                           ▼               ▼
    /// ┌ ─ ─ ─ ─ ─ ─               ┌ ─ ─ ─ ─ ─ ┐   ┌ ─ ─ ─ ─ ─ ┐
    ///    Child A   │              |  Child B  │   |  Child C  │
    /// └ ─ ─ ─ ─ ─ ─               └ ─ ─ ─ ── -    └ ─ ─ ─ ── ─
    /// ```
    /// Note that not all connectors can have their nodes' field selection updated.
    /// This is only possible when the parent node _can_ fulfill the selection set.
    /// In the case of updates and inserts, for instance, only connectors supporting `InsertReturning` and `UpdateReturning` can do it,
    /// or else they're only able to return the primary identifier of the model inserted or updated.
    fn normalize_data_dependencies(&mut self, capabilities: ConnectorCapabilities) -> QueryGraphResult<()> {
        let unsatisfied_deps = self.find_unsatisfied_dependencies();

        for (node, identifiers) in unsatisfied_deps {
            let query = self
                .node_content_mut(&node)
                .and_then(|node| node.as_query_mut())
                .unwrap();

            // If the connector does not support returning more than the primary identifier for an update,
            // do not update the selection set.
            if query.is_update_one() && !capabilities.contains(ConnectorCapability::UpdateReturning) {
                continue;
            }

            // If the connector does not support returning more than the primary identifier for a create,
            // do not update the selection set.
            if query.is_create_one() && !capabilities.contains(ConnectorCapability::InsertReturning) {
                continue;
            }

            // If the connector does not support returning more than the primary identifier for a delete,
            // do not update the selection set.
            if query.is_delete_one() && !capabilities.contains(ConnectorCapability::DeleteReturning) {
                continue;
            }

            trace!(
                "Query {:?} does not return requested selection {:?} and will be updated.",
                query,
                identifiers.prisma_names().collect::<Vec<_>>()
            );

            query.satisfy_dependency(identifiers);
        }

        Ok(())
    }

    /// Traverses the query graph and finds the query nodes that don't fulfill their children data dependencies.
    /// We determine that based on incoming `ProjectedDataDependency` edge transformers, as those hold the `FieldSelection`s
    /// that all records of the source result need to contain in order to satisfy dependencies.
    fn find_unsatisfied_dependencies(&self) -> Vec<(NodeRef, FieldSelection)> {
        self.graph
            .node_indices()
            .filter_map(|ix| {
                let node = NodeRef { node_ix: ix };

                if let Node::Query(q) = self.node_content(&node).unwrap() {
                    let edges = self.outgoing_edges(&node);
                    let unsatisfied_dependencies: Vec<_> = edges
                        .into_iter()
                        .filter_map(|edge| match self.edge_content(&edge).unwrap() {
                            QueryGraphDependency::ProjectedDataDependency(ref requested_selection, _, _)
                            | QueryGraphDependency::ProjectedDataSinkDependency(ref requested_selection, _, _)
                                if !q.satisfies(requested_selection) =>
                            {
                                Some(requested_selection.clone())
                            }
                            _ => None,
                        })
                        .collect();

                    if unsatisfied_dependencies.is_empty() {
                        None
                    } else {
                        Some((node, FieldSelection::union(unsatisfied_dependencies)))
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

pub trait ToGraphviz {
    fn to_graphviz(&self) -> String;
}

impl ToGraphviz for QueryGraph {
    fn to_graphviz(&self) -> String {
        let label_from_node = |idx: usize, node: &Node| {
            format!(
                "(n{})\\n{}\\l",
                idx,
                node.to_graphviz().replace('\"', "\\\"").replace('\n', "\\l")
            )
        };

        let nodes = self
            .graph
            .node_indices()
            .map(|idx| (idx, self.graph.node_weight(idx).unwrap().borrow().unwrap()))
            .map(|(idx, node)| {
                if self.is_result_node(&NodeRef { node_ix: idx }) {
                    format!(
                        "    {} [label=\"{}\", fillcolor=blue, style=filled, shape=rectangle, fontcolor=white]",
                        idx.index(),
                        label_from_node(idx.index(), node)
                    )
                } else if self.root_nodes().any(|root_node| root_node == NodeRef { node_ix: idx }) {
                    format!(
                        "    {} [label=\"{}\", fillcolor=red, style=filled, shape=rectangle, fontcolor=white]",
                        idx.index(),
                        label_from_node(idx.index(), node)
                    )
                } else {
                    format!(
                        "    {} [label=\"{}\", shape=rectangle]",
                        idx.index(),
                        label_from_node(idx.index(), node)
                    )
                }
            })
            .join("\n");

        let edges = self
            .graph
            .edge_references()
            .map(|edge| {
                let idx = edge.id();
                let edge_content = self.graph.edge_weight(idx).unwrap().borrow().unwrap();

                format!(
                    "    {} -> {} [label=\"(e{}) {}\"]",
                    self.graph.to_index(edge.source()),
                    self.graph.to_index(edge.target()),
                    idx.index(),
                    edge_content.to_string().replace('\"', "\\\"")
                )
            })
            .join("\n");

        format!("digraph {{\n{nodes}\n{edges}\n}}")
    }
}
