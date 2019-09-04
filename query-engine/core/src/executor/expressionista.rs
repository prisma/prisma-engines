use super::*;
use crate::QueryGraph;
use connector::ReadQuery;

pub struct Expressionista;

impl Expressionista {
    pub fn translate(graph: QueryGraph) -> Expression {
        let root_nodes: Vec<Node> = graph.root_nodes();
        let expressions = root_nodes
            .into_iter()
            .map(|root_node| Self::build_expression(&graph, root_node, None))
            .collect();

        Expression::Sequence { seq: expressions }
    }

    fn build_expression(graph: &QueryGraph, node: Node, parent_edge: Option<Edge>) -> Expression {
        let child_edges = graph.edges_for(&node, EdgeDirection::Outgoing);
        let query = graph.remove_node(node);
        let exp = Self::query_expression(graph, parent_edge, query);

        // Writes before reads
        let (write_edges, read_edges): (Vec<_>, Vec<_>) = child_edges.into_iter().partition(|child_edge| match &*graph
            .edge_content(&child_edge)
        {
            EdgeContent::Write(_, _) => true,
            EdgeContent::Read(_) => false,
        });

        let mut expressions: Vec<_> = write_edges
            .into_iter()
            .map(|child_edge| Self::build_expression(graph, graph.edge_target(&child_edge), Some(child_edge)))
            .collect();

        let mut read_expressions: Vec<_> = read_edges
            .into_iter()
            .map(|child_edge| Self::build_expression(graph, graph.edge_target(&child_edge), Some(child_edge)))
            .collect();

        expressions.append(&mut read_expressions);

        if expressions.is_empty() {
            exp
        } else {
            Expression::Let {
                bindings: vec![Binding {
                    name: "parent".to_owned(),
                    exp,
                }],
                expressions: expressions,
            }
        }
    }

    fn query_expression(graph: &QueryGraph, parent_edge: Option<Edge>, query: Query) -> Expression {
        match (parent_edge, query) {
            (None, Query::Write(wq)) => Expression::Write { write: wq },
            (None, Query::Read(rq)) => Expression::Read { read: rq },

            (Some(child_edge), Query::Write(wq)) => match graph.remove_edge(child_edge) {
                EdgeContent::Write(_, EdgeOperation::DependentId(f)) => Expression::Func {
                    func: Box::new(|env: Env| {
                        let parent_result = env.get("parent").unwrap();
                        let parent_id = parent_result.as_id();
                        let query = f(wq, parent_id);

                        Expression::Write { write: query }
                    }),
                },
                _ => unreachable!(),
            },

            (Some(child_edge), Query::Read(rq)) => match rq {
                ReadQuery::RecordQuery(_) => match graph.remove_edge(child_edge) {
                    EdgeContent::Read(EdgeOperation::DependentId(f)) => Expression::Func {
                        func: Box::new(|env: Env| {
                            let parent_result = env.get("parent").unwrap();
                            let parent_id = parent_result.as_id();
                            let query = f(rq, parent_id);

                            Expression::Read { read: query }
                        }),
                    },
                    _ => unreachable!(),
                },
                _ => unimplemented!(),
            },
        }
    }
}
