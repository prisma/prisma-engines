use super::*;
use crate::query_graph::{Edge, Node, QueryGraph};

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
        let is_result = graph.is_result_node(&node);

        // Child edges are ordered, execution rule is left to right in the graph, unless other rules override.
        let mut child_edges = graph.outgoing_edges(&node);
        let query = graph.pluck_node(node);
        let exp = Self::query_expression(graph, parent_edge, query);

        // Split the children into a set of siblings to be executed before the result and one with/after the result subgraph.
        let result_position = child_edges.iter().position(|edge| {
            let target = graph.edge_target(edge);
            graph.subgraph_contains_result(&target)
        });

        let (before, after) = match result_position {
            Some(pos) => (child_edges.split_off(pos), child_edges),
            None => (child_edges, vec![]),
        };

        let mut before_result: Vec<_> = before
            .into_iter()
            .map(|child_edge| Self::build_expression(graph, graph.edge_target(&child_edge), Some(child_edge)))
            .collect();

        let mut after_result: Vec<_> = after
            .into_iter()
            .map(|child_edge| Self::build_expression(graph, graph.edge_target(&child_edge), Some(child_edge)))
            .collect();

        // Append a let binding to bind the result to a variable and allow it to be returned last to propagate back up the call chain.
        // By the virtue of the split, the subgraph root node that contains the result is first in the `after_result` vector.
        // If `after_vector` contains only the expression of the result graph, just append it to the end - no additional bindings
        // are required.
        let mut expressions = if after_result.len() > 1 {
            let head = after_result.split_off(0).pop().unwrap();

            after_result.push(Expression::Get {
                binding_name: "result".to_owned(),
            });

            before_result.push(Expression::Let {
                bindings: vec![Binding {
                    name: "result".to_owned(),
                    exp: head,
                }],
                expressions: after_result,
            });

            before_result
        } else {
            before_result.append(&mut after_result);
            before_result
        };

        // // Writes before reads
        // let (write_edges, read_edges): (Vec<_>, Vec<_>) = child_edges.into_iter().partition(|child_edge| match &*graph
        //     .edge_content(&child_edge)
        // {
        //     QueryDependency::Write(_, _) => true,
        //     QueryDependency::Read(_) => false,
        // });

        // let mut expressions: Vec<_> = write_edges
        //     .into_iter()
        //     .map(|child_edge| Self::build_expression(graph, graph.edge_target(&child_edge), Some(child_edge)))
        //     .collect();

        // let mut read_expressions: Vec<_> = read_edges
        //     .into_iter()
        //     .map(|child_edge| Self::build_expression(graph, graph.edge_target(&child_edge), Some(child_edge)))
        //     .collect();

        // expressions.append(&mut read_expressions);

        if expressions.is_empty() {
            exp
        } else {
            // Add a final statement to the evaluation if the current node has child nodes and is supposed to be the
            // final result, to make sure it propagates upwards.
            if is_result {
                expressions.push(Expression::Get {
                    binding_name: "parent".to_owned(),
                });
            };

            Expression::Let {
                bindings: vec![Binding {
                    name: "parent".to_owned(),
                    exp,
                }],
                expressions,
            }
        }
    }

    fn query_expression(graph: &QueryGraph, parent_edge: Option<Edge>, query: Query) -> Expression {
        match (parent_edge, query) {
            (None, Query::Write(wq)) => Expression::Write { write: wq },
            (None, Query::Read(rq)) => Expression::Read { read: rq },

            (Some(child_edge), Query::Write(wq)) => match graph.pluck_edge(child_edge) {
                QueryDependency::Write(_, DependencyType::ParentId(f)) => Expression::Func {
                    func: Box::new(|env: Env| {
                        let parent_result = env.get("parent").unwrap();
                        let parent_id = parent_result.as_id();
                        let query = f(wq, parent_id);

                        Expression::Write { write: query }
                    }),
                },

                QueryDependency::Write(_, DependencyType::ExecutionOrder) => Expression::Write { write: wq },
                _ => unreachable!(),
            },

            (Some(child_edge), Query::Read(rq)) => match graph.pluck_edge(child_edge) {
                QueryDependency::Read(DependencyType::ParentId(f)) => Expression::Func {
                    func: Box::new(|env: Env| {
                        let parent_result = env.get("parent").unwrap();
                        let parent_id = parent_result.as_id();
                        let query = f(rq, parent_id);

                        Expression::Read { read: query }
                    }),
                },
                _ => unreachable!(),
            },
        }
    }
}
