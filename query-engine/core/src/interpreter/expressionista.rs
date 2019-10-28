use super::{expression::*, Env, InterpretationResult, InterpreterError};
use crate::{query_graph::*, Query};
use std::convert::TryInto;

pub struct Expressionista;

impl Expressionista {
    pub fn translate(mut graph: QueryGraph) -> InterpretationResult<Expression> {
        graph
            .root_nodes()
            .into_iter()
            .map(|root_node| Self::build_expression(&mut graph, &root_node, vec![]))
            .collect::<InterpretationResult<Vec<Expression>>>()
            .map(|res| Expression::Sequence { seq: res })
    }

    fn build_expression(
        graph: &mut QueryGraph,
        node: &NodeRef,
        mut parent_edges: Vec<EdgeRef>,
    ) -> InterpretationResult<Expression> {
        match graph
            .node_content(node)
            .expect(&format!("Node content {} was empty", node.id()))
        {
            Node::Query(_) => Self::build_query_expression(graph, node, parent_edges),
            Node::Flow(_) => Self::build_flow_expression(graph, node, parent_edges.pop()),
        }
    }

    fn build_query_expression(
        graph: &mut QueryGraph,
        node: &NodeRef,
        parent_edges: Vec<EdgeRef>,
    ) -> InterpretationResult<Expression> {
        // Child edges are ordered, evaluation order is low to high in the graph, unless other rules override.
        let mut direct_children = graph.direct_child_pairs(&node);

        // Find the positions of all result returning graph nodes.
        let result_positions: Vec<usize> = direct_children
            .iter()
            .enumerate()
            .filter_map(|(ix, (_, child_node))| {
                if graph.subgraph_contains_result(&child_node) {
                    Some(ix)
                } else {
                    None
                }
            })
            .collect();

        let result_subgraphs: Vec<(EdgeRef, NodeRef)> = result_positions
            .into_iter()
            .map(|pos| direct_children.remove(pos))
            .collect();

        // Because we split from right to left, everything remaining in `direct_children`
        // doesn't belong into results, and is executed before all result scopes.
        let mut expressions: Vec<Expression> = direct_children
            .into_iter()
            .map(|(_, node)| {
                let edges = graph.incoming_edges(&node);
                Self::build_expression(graph, &node, edges)
            })
            .collect::<InterpretationResult<Vec<Expression>>>()?;

        // Fold result scopes into one expression.
        if result_subgraphs.len() > 0 {
            let result_exp = Self::fold_result_scopes(graph, result_subgraphs)?;
            expressions.push(result_exp);
        }

        let is_result = graph.is_result_node(&node);
        let node_id = node.id();
        let query: Query = graph.pluck_node(&node).try_into()?;
        let expr = Self::query_expression(graph, parent_edges, query);

        if expressions.is_empty() {
            Ok(expr)
        } else {
            let node_binding_name = node_id.clone();

            // Add a final statement to the evaluation if the current node has child nodes and is supposed to be the
            // final result, to make sure it propagates upwards.
            if is_result {
                expressions.push(Expression::Get {
                    binding_name: node_binding_name.clone(),
                });
            }

            Ok(Expression::Let {
                bindings: vec![Binding {
                    name: node_binding_name,
                    expr,
                }],
                expressions,
            })
        }
    }

    /// Returns an `Expression::Query` directly, or indirectly via `Expression::Func`.
    fn query_expression(graph: &mut QueryGraph, parent_edges: Vec<EdgeRef>, query: Query) -> Expression {
        if parent_edges.is_empty() {
            Expression::Query { query }
        } else {
            // Collect all parent ID dependency tuples (transformers).
            let parent_id_deps: Vec<(String, Box<_>)> = parent_edges
                .into_iter()
                .filter_map(|edge| match graph.pluck_edge(&edge) {
                    QueryGraphDependency::ParentIds(f) => {
                        let parent_binding_name = graph.edge_source(&edge).id();
                        Some((parent_binding_name, f))
                    }
                    _ => None,
                })
                .collect();

            // If there is at least one parent ID dependency we build a func to run the transformer(s),
            // else just render a flat query expression.
            if parent_id_deps.is_empty() {
                Expression::Query { query }
            } else {
                Expression::Func {
                    func: Box::new(move |env: Env| {
                        // Run transformers in order on the query to retrieve the final, transformed, query.
                        let query: InterpretationResult<Query> =
                            parent_id_deps
                                .into_iter()
                                .try_fold(query, |query, (parent_binding_name, f)| {
                                    let binding = match env.get(&parent_binding_name) {
                                        Some(binding) => Ok(binding),
                                        None => Err(InterpreterError::EnvVarNotFound(format!(
                                            "Expected parent binding '{}' to be present.",
                                            parent_binding_name
                                        ))),
                                    }?;

                                    let parent_ids = match binding.as_ids() {
                                        Some(ids) => Ok(ids),
                                        None => Err(InterpreterError::InterpretationError(format!(
                                        "Invalid parent result: Unable to transform binding '{}' into a set of IDs.",
                                        parent_binding_name
                                    ))),
                                    }?;

                                    let query: Query = f(query.into(), parent_ids)?.try_into()?;

                                    Ok(query)
                                });

                        Ok(Expression::Query { query: query? })
                    }),
                }
            }
        }
    }

    fn build_flow_expression(
        graph: &mut QueryGraph,
        node: &NodeRef,
        parent_edge: Option<EdgeRef>,
    ) -> InterpretationResult<Expression> {
        let flow: Flow = graph.pluck_node(node).try_into()?;

        match flow {
            Flow::If(_) => {
                let child_pairs = graph.child_pairs(node);

                // Graph validation guarantees this succeeds.
                let (mut then_pair, mut else_pair): (Vec<(EdgeRef, NodeRef)>, Vec<(EdgeRef, NodeRef)>) = child_pairs
                    .into_iter()
                    .partition(|(edge, _)| match graph.edge_content(&edge).unwrap() {
                        QueryGraphDependency::Then => true,
                        QueryGraphDependency::Else => false,
                        _ => unreachable!(),
                    });

                let then_pair = then_pair.pop().unwrap();
                let else_pair = else_pair.pop();

                // Build expressions for both arms. They are treated as separate root nodes.
                let then_expr = Self::build_expression(graph, &then_pair.1, graph.incoming_edges(&then_pair.1))?;
                let else_expr = else_pair
                    .into_iter()
                    .map(|(_, node)| Self::build_expression(graph, &node, graph.incoming_edges(&node)))
                    .collect::<InterpretationResult<Vec<Expression>>>()?;

                Ok(match parent_edge {
                    Some(p) => {
                        match graph.pluck_edge(&p) {
                            QueryGraphDependency::ParentIds(f) => {
                                let parent_binding_name = graph.edge_source(&p).id();
                                Expression::Func {
                                    func: Box::new(move |env| {
                                        let binding = match env.get(&parent_binding_name) {
                                            Some(binding) => Ok(binding),
                                            None => Err(InterpreterError::EnvVarNotFound(format!(
                                                "Expected parent binding '{}' to be present.",
                                                parent_binding_name
                                            ))),
                                        }?;

                                        let parent_ids = match binding.as_ids() {
                                            Some(ids) => Ok(ids),
                                            None => Err(InterpreterError::InterpretationError(format!("Invalid parent result: Unable to transform binding '{}' into a set of IDs.", parent_binding_name))),
                                        }?;

                                        // Todo: This still needs some interface polishing...
                                        let flow: Flow = f(flow.into(), parent_ids)?.try_into()?;
                                        match flow {
                                            Flow::If(cond_fn) => {
                                                // Todo: Maybe this construct points to a misconception: That the if node needs to be actually in the program.
                                                // At this point here, we could evaluate the condition already and decide which branch to return.
                                                Ok(Expression::If {
                                                    func: cond_fn,
                                                    then: vec![then_expr],
                                                    else_: else_expr,
                                                })
                                            }
                                        }
                                    }),
                                }
                            }
                            _ => unimplemented!(),
                        }
                    }

                    None => unimplemented!(),
                })
            }
        }
    }

    fn fold_result_scopes(
        graph: &mut QueryGraph,
        result_subgraphs: Vec<(EdgeRef, NodeRef)>,
    ) -> InterpretationResult<Expression> {
        let bindings: Vec<Binding> = result_subgraphs
            .into_iter()
            .map(|(_, node)| {
                let name = node.id();
                let edges = graph.incoming_edges(&node);
                let expr = Self::build_expression(graph, &node, edges)?;

                Ok(Binding { name, expr })
            })
            .collect::<InterpretationResult<Vec<Binding>>>()?;

        let result_binding_names = bindings.iter().map(|b| b.name.clone()).collect();

        Ok(Expression::Let {
            bindings,
            expressions: vec![Expression::GetFirstNonEmpty {
                binding_names: result_binding_names,
            }],
        })
    }
}
