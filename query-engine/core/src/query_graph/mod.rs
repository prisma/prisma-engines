//! Query graph abstraction for simple high-level query representation and manipulation.
//! Wraps Petgraph crate graph.
//!
//! Considered not stable. Will change in the future.
mod error;
mod formatters;
// mod guard;
mod transformers;

#[cfg(test)]
mod tests;

pub use error::*;
pub use formatters::*;
pub use transformers::*;

use crate::Query;
use petgraph::{graph::*, visit::EdgeRef as PEdgeRef, *};
use prisma_models::{ModelProjection, RelationFieldRef};

pub type QueryGraphResult<T> = std::result::Result<T, QueryGraphError>;

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

    /// Designates the node that is returning the result of the entire QueryGraph.
    /// If no node is set, the interpretation will take the result of the
    /// last statement derived from the graph.
    result_node: Option<NodeIndex>,

    /// Pairs of nodes marked for parent child swap.
    /// The first `NodeRef` of the tuple is the parent, the second one the child.
    /// The child will become the parents new parent when the graph is finalized.
    /// More docs can be found on `swap_marked`.
    marked_node_pairs: Vec<(NodeRef, NodeRef)>,

    /// State marker designating whether or not the graph has undergone finalization.
    finalized: bool,

    /// Marks the entire query graph for execution in a single transaction.
    transactional: bool,
}

/// Implementation detail of the QueryGraph.
type InnerGraph = Graph<Query, Option<QueryDependency>>;

/// A `QueryDependency` expresses that one query requires a certain set of fields to be returned from the
/// dependent query in order to be able to satisfy it's execution requirements.
#[derive(Debug)]
pub enum QueryDependency {
    /// Filter the dependent query by the data of the parent query.
    InjectFilter(DependencyType),

    /// Inject data into the dependent query from the parent query.
    InjectData(DependencyType),
}

impl QueryDependency {
    pub fn flip(self) -> Self {
        match self {
            Self::InjectFilter(typ) => Self::InjectFilter(typ.flip()),
            Self::InjectData(typ) => Self::InjectData(typ.flip()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DependencyType {
    /// Depend on a specific combination of fields of the parent query.
    Projection(ModelProjection),

    /// Depend on the linking fields of the contained relation field of the parent query (i.e. opposing model).
    Relation(RelationFieldRef),
}

impl DependencyType {
    pub fn flip(self) -> Self {
        match self {
            Self::Relation(rf) => Self::Relation(rf.related_field()),
            x => x,
        }
    }
}

impl Into<ModelProjection> for DependencyType {
    fn into(self) -> ModelProjection {
        match self {
            DependencyType::Projection(p) => p,
            DependencyType::Relation(r) => r.linking_fields(),
        }
    }
}

impl From<ModelProjection> for DependencyType {
    fn from(p: ModelProjection) -> Self {
        Self::Projection(p)
    }
}

impl From<RelationFieldRef> for DependencyType {
    fn from(p: RelationFieldRef) -> Self {
        Self::Relation(p)
    }
}

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

        Ok(())
    }

    /// Marks given node as a (possible) result node of the query graph.
    pub fn mark_result_node(&mut self, node: &NodeRef) {
        self.result_node = Some(node.node_ix.clone());
    }

    /// Checks if the given node is marked as the result node in the graph.
    pub fn is_result_node(&self, node: &NodeRef) -> bool {
        self.result_node
            .as_ref()
            .map(|ix| ix.index() == node.node_ix.index())
            .unwrap_or(false)
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

    /// Returns the root node of the graph.
    /// A root node is defined by having no incoming edges.
    /// More than one root node found results in a panic.
    pub fn root_node(&self) -> NodeRef {
        let graph = &self.graph;

        let mut nodes = graph
            .node_indices()
            .filter_map(|ix| {
                if let Some(_) = graph.edges_directed(ix, Direction::Incoming).next() {
                    None
                } else {
                    Some(ix)
                }
            })
            .map(|node_ix: NodeIndex| NodeRef { node_ix })
            .collect::<Vec<_>>();

        if nodes.len() != 1 {
            panic!(format!(
                "Expected query graph to contain exactly one root node, found: {} ({:?})",
                nodes.len(),
                nodes,
            ))
        } else {
            nodes.pop().unwrap()
        }
    }

    /// Mark the query graph to run inside of a transaction.
    pub fn flag_transactional(&mut self) {
        self.transactional = true;
    }

    /// If true, the graph should be executed inside of a transaction.
    pub fn transactional(&self) -> bool {
        self.transactional
    }

    /// Creates a node with given query in the graph and returns a `NodeRef` to the created node.
    pub fn create_node(&mut self, q: Query) -> NodeRef {
        let node_ix = self.graph.add_node(q);

        NodeRef { node_ix }
    }

    /// Creates an edge with given `content`, originating from node `from` and pointing to node `to`.
    /// Returns an `EdgeRef` to the newly added edge.
    pub fn create_edge(&mut self, from: &NodeRef, to: &NodeRef, content: Option<QueryDependency>) -> EdgeRef {
        let edge_ix = self.graph.add_edge(from.node_ix, to.node_ix, content);

        EdgeRef { edge_ix }
    }

    /// Returns a reference to the content of `node`, if the content is still present.
    pub fn node_content(&self, node: &NodeRef) -> &Query {
        self.graph.node_weight(node.node_ix).unwrap()
    }

    /// Returns a reference to the content of `edge`, if the content is still present.
    pub fn edge_content(&self, edge: &EdgeRef) -> &Option<QueryDependency> {
        self.graph.edge_weight(edge.edge_ix).unwrap()
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

    // /// Removes the edge from the graph but leaves the graph intact by keeping the empty
    // /// edge in the graph by plucking the content of the edge, but not the edge itself.
    // pub fn pluck_edge(&mut self, edge: &EdgeRef) -> Option<RelationFieldRef> {
    //     self.graph.edge_weight_mut(edge.edge_ix).unwrap().unset()
    // }

    // /// Removes the node from the graph but leaves the graph intact by keeping the empty
    // /// node in the graph by plucking the content of the node, but not the node itself.
    // pub fn pluck_node(&mut self, node: &NodeRef) -> Query {
    //     self.graph.node_weight_mut(node.node_ix).unwrap()
    // }

    /// Completely removes the edge from the graph, returning it's content.
    /// This operation is destructive on the underlying graph and invalidates references.
    pub fn remove_edge(&mut self, edge: EdgeRef) -> Option<QueryDependency> {
        self.graph.remove_edge(edge.edge_ix).unwrap()
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
    /// exection order, and no relational dependency.
    ///
    /// An edge existing between `parent` and `child` will change direction and will point from `child` to `parent` instead,
    /// flipping the query dependency (e.g. changing the relation field to the opposing one), if any.
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
    fn swap_marked(&mut self) -> QueryGraphResult<()> {
        if self.marked_node_pairs.len() > 0 {
            trace!("[Graph][Swap] Before shape: {}", self);
        }

        let mut marked = std::mem::replace(&mut self.marked_node_pairs, vec![]);
        marked.reverse(); // [DTODO]: Marked operation order is currently breaking if done bottom-up. Investigate how to fix it.

        for (parent_node, child_node) in marked {
            // All parents of `parent_node` are becoming a parent of `child_node` as well.
            let parent_edges = self.incoming_edges(&parent_node);
            for parent_edge in parent_edges {
                let parent_of_parent_node = self.edge_source(&parent_edge);

                trace!(
                    "[Graph][Swap] Connecting parent of parent {} with child {}",
                    parent_of_parent_node.id(),
                    child_node.id()
                );

                self.create_edge(&parent_of_parent_node, &child_node, None);
            }

            // Find existing edge between parent and child. Can only be one at most.
            let existing_edge = self
                .graph
                .find_edge(parent_node.node_ix, child_node.node_ix)
                .map(|edge_ix| EdgeRef { edge_ix });

            // Remove edge and reinsert edge in reverse with the content reversed as well, if possible.
            if let Some(edge) = existing_edge {
                let content = self.remove_edge(edge).map(|dep| dep.flip());

                self.create_edge(&child_node, &parent_node, content);
            }
        }

        Ok(())
    }

    //   /// Traverses the query graph and checks if reloads of nodes are necessary.
    //   /// Whether or not a node needs to be reloaded is determined based on the
    //   /// outgoing edges of parent-projection-based transformers, as those hold the `ModelProjection`s
    //   /// all records of the parent result need to contain in order to satisfy dependencies.
    //   ///
    //   /// ## Example
    //   /// Given a query graph, where 3 children require different set of fields ((A, B), (B, C), (A, D))
    //   /// to execute their dependent operations:
    //   /// ```text
    //   /// ┌ ─ ─ ─ ─ ─ ─
    //   ///     Parent   │─────────┬───────────────┐
    //   /// └ ─ ─ ─ ─ ─ ─          │               │
    //   ///        │               │               │
    //   ///     (A, B)          (B, C)           (A, D)
    //   ///        │               │               │
    //   ///        ▼               ▼               ▼
    //   /// ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─
    //   ///    Child A   │     Child B   │     Child C   │
    //   /// └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─
    //   /// ```
    //   /// However, `Parent` only returns `(A, B)`, for example, because that's the primary ID of the parent model
    //   /// and `Parent` is an operation that only returns IDs (e.g. update, updateMany).
    //   ///
    //   /// In order to satisfy children B and C, the graph is altered by this post-processing call:
    //   /// ```text
    //   /// ┌ ─ ─ ─ ─ ─ ─
    //   ///     Parent   │
    //   /// └ ─ ─ ─ ─ ─ ─
    //   ///        │
    //   ///     (A, B) (== Primary ID)
    //   ///        │
    //   ///        ▼
    //   /// ┌────────────┐
    //   /// │   Reload   │─────────┬───────────────┐
    //   /// └────────────┘         │               │
    //   ///        │               │               │
    //   ///     (A, B)          (B, C)           (A, D)
    //   ///        │               │               │
    //   ///        ▼               ▼               ▼
    //   /// ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─   ┌ ─ ─ ─ ─ ─ ─
    //   ///    Child A   │     Child B   │     Child C   │
    //   /// └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─   └ ─ ─ ─ ─ ─ ─
    //   /// ```
    //   ///
    //   /// The edges from `Parent` to all dependent children are removed from the graph and reinserted in order
    //   /// on the reload node.
    //   ///
    //   /// The `Reload` node is always a find many query.
    //   /// Unwraps are safe because we're operating on the unprocessed state of the graph (`Expressionista` changes that).
    //   // fn insert_reloads(&mut self) -> QueryGraphResult<()> {
    //     let reloads: Vec<(NodeRef, ModelRef, Vec<(EdgeRef, ModelProjection)>)> = self
    //         .graph
    //         .node_indices()
    //         .filter_map(|ix| {
    //             let node = NodeRef { node_ix: ix };

    //             if let Node::Query(q) = self.node_content(&node).unwrap() {
    //                 let edges = self.outgoing_edges(&node);

    //                 let unsatisfied_edges: Vec<_> = edges
    //                     .into_iter()
    //                     .filter_map(|edge| match self.edge_content(&edge).unwrap() {
    //                         QueryGraphDependency::ParentProjection(ref requested_projection, _)
    //                             if !q.returns(requested_projection) =>
    //                         {
    //                             trace!(
    //                                 "Query {:?} does not return requested projection {:?} and will be reloaded.",
    //                                 q,
    //                                 requested_projection.names().collect::<Vec<_>>()
    //                             );
    //                             Some((edge, requested_projection.clone()))
    //                         }
    //                         _ => None,
    //                     })
    //                     .collect();

    //                 if unsatisfied_edges.is_empty() {
    //                     None
    //                 } else {
    //                     Some((node, q.model(), unsatisfied_edges))
    //                 }
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect();

    //     for (node, model, edges) in reloads {
    //         // Create reload node and connect it to the `node`.
    //         let primary_model_id = model.primary_identifier();
    //         let (edges, mut identifiers): (Vec<_>, Vec<_>) = edges.into_iter().unzip();
    //         identifiers.push(primary_model_id.clone());

    //         let read_query = ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
    //             name: "reload".into(),
    //             alias: None,
    //             model,
    //             args: QueryArguments::default(),
    //             selected_fields: ModelProjection::union(identifiers),
    //             nested: vec![],
    //             selection_order: vec![],
    //         });

    //         let query = Query::Read(read_query);
    //         let reload_node = self.create_node(query);

    //         self.create_edge(
    //             &node,
    //             &reload_node,
    //             QueryGraphDependency::ParentProjection(
    //                 primary_model_id,
    //                 Box::new(|mut reload_node, parent_projections| {
    //                     if let Node::Query(Query::Read(ReadQuery::ManyRecordsQuery(ref mut mr))) = reload_node {
    //                         mr.set_filter(parent_projections.filter());
    //                     }

    //                     Ok(reload_node)
    //                 }),
    //             ),
    //         )?;

    //         // Remove unsatisfied edges from node, reattach them to the reload node
    //         for edge in edges {
    //             let target = self.edge_target(&edge);
    //             let content = self.remove_edge(edge).unwrap();

    //             self.create_edge(&reload_node, &target, content)?;
    //         }
    //     }

    //     Ok(())
}
