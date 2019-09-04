///! Query graph abstraction for simple high-level query representation
///! and manipulation.
use connector::*;
use petgraph::{graph::*, visit::EdgeRef, *};
use prisma_models::{PrismaValue, RelationFieldRef};
use std::cell::RefCell;
use std::ops::Deref;

/// Implementation detail of the query graph.
type InnerGraph = Graph<Query, EdgeContent>;
// type InnerGraph = Graph<Guard<Query>, Guard<EdgeContent>>;

/// Workaround to keep the graph stable during removals
// enum Guard<T> {
//     Empty,
//     Set(T),
// }

#[derive(Default)]
pub struct QueryGraph {
    graph: RefCell<InnerGraph>,
}

pub struct Node {
    pub(self) node_ix: NodeIndex,
}

pub enum EdgeDirection {
    Outgoing,
    Incoming,
}

pub struct Edge {
    pub(self) edge_ix: EdgeIndex,
}

/// Read / Write distinction is only really important for ordering in the interpreter...
/// we should try to get rid of that.
/// Another major factor is the RelationFieldRef, which is required in the graph transformation.
pub enum EdgeContent {
    Write(RelationFieldRef, EdgeOperation<WriteQuery>),
    Read(EdgeOperation<ReadQuery>),
}

pub enum EdgeOperation<T> {
    /// Performs a transformation on a query based on the parent ID (PrismaValue)
    DependentId(Box<dyn FnOnce(T, PrismaValue) -> T>),
}

impl QueryGraph {
    pub fn new() -> Self {
        Self {
            graph: RefCell::new(InnerGraph::new()),
        }
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

    pub fn create_node(&self, query: Query) -> Node {
        let node_ix = self.graph.borrow_mut().add_node(query);

        Node { node_ix }
    }

    pub fn create_edge(&self, from: &Node, to: &Node, content: EdgeContent) -> Edge {
        let edge_ix = self.graph.borrow_mut().add_edge(from.node_ix, to.node_ix, content);

        Edge { edge_ix }
    }

    pub fn node_content(&self, node: &Node) -> impl Deref<Target = Query> + '_ {
        std::cell::Ref::map(self.graph.borrow(), |g| g.node_weight(node.node_ix).unwrap())
    }

    pub fn edge_content(&self, edge: &Edge) -> impl Deref<Target = EdgeContent> + '_ {
        std::cell::Ref::map(self.graph.borrow(), |g| g.edge_weight(edge.edge_ix).unwrap())
    }

    pub fn edge_source(&self, edge: &Edge) -> Node {
        let (node_ix, _) = self.graph.borrow().edge_endpoints(edge.edge_ix).unwrap();

        Node { node_ix }
    }

    pub fn edge_target(&self, edge: &Edge) -> Node {
        let (_, node_ix) = self.graph.borrow().edge_endpoints(edge.edge_ix).unwrap();

        Node { node_ix }
    }

    pub fn edges_for(&self, node: &Node, direction: EdgeDirection) -> Vec<Edge> {
        let direction = match direction {
            EdgeDirection::Outgoing => Direction::Outgoing,
            EdgeDirection::Incoming => Direction::Incoming,
        };

        self.graph
            .borrow()
            .edges_directed(node.node_ix, direction)
            .map(|edge| Edge { edge_ix: edge.id() })
            .collect()
    }

    pub fn remove_edge(&self, edge: Edge) -> EdgeContent {
        self.graph.borrow_mut().remove_edge(edge.edge_ix).unwrap()
    }

    pub fn remove_node(&self, node: Node) -> Query {
        self.graph.borrow_mut().remove_node(node.node_ix).unwrap()
    }

    /// Current way to fix inconsistencies in the graph.
    pub fn transform(self) -> Self {
        let mut graph = self.graph.borrow_mut();
        let candidates: Vec<EdgeIndex> = graph
            .raw_edges()
            .into_iter()
            .filter_map(|edge| {
                let parent = graph.node_weight(edge.source()).unwrap();
                let child = graph.node_weight(edge.target()).unwrap();
                let edge_index = graph.find_edge(edge.source(), edge.target()).unwrap();

                match (parent, child) {
                    (
                        Query::Write(WriteQuery::Root(RootWriteQuery::CreateRecord(_))),
                        Query::Write(WriteQuery::Root(RootWriteQuery::CreateRecord(_))),
                    ) => {
                        let relation_field: &RelationFieldRef = match &edge.weight {
                            EdgeContent::Write(rf, _) => rf,
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
            let edge = graph.remove_edge(edge_index).unwrap();

            // Warning: This assumes that the EdgeOperation is also flippable.
            if let EdgeContent::Write(rf, op) = edge {
                graph.add_edge(child, parent, EdgeContent::Write(rf.related_field(), op));
            }
        });

        drop(graph);

        self
    }
}
