use super::*;
use crate::query_graph::{EdgeRef, Node, NodeRef, QueryGraph};
use std::convert::TryInto;

pub struct Expressionista;

impl Expressionista {
    pub fn translate(mut graph: QueryGraph) -> QueryExecutionResult<Expression> {
        graph.root_nodes()
            .into_iter()
            .map(|root_node| Self::build_expression(&mut graph, root_node, None))
            .collect::<QueryExecutionResult<Vec<Expression>>>()
            .map(|res| Expression::Sequence { seq: res })
    }

    fn build_expression(graph: &mut QueryGraph, node: NodeRef, parent_edge: Option<EdgeRef>) -> QueryExecutionResult<Expression> {
        // Graph invariants (TODO put them into code in the graph impl):
        // - Directed, acyclic.
        // - Node IDs are unique and stable, as they are used to bind variables in the scopes.
        // - At most one node is designated as result node.
        // - Multiple paths in the graph may point to the result node. The first non-empty (ExpressionResult::Empty) return value of those path wins.
        // - Currently, Nodes are allowed to have multiple parents, but the following invariants apply:
        //   * They may only refer to their parent and / or one of its direct ancestors.
        //   * Note: This rule guarantees that the dependent ancestor node result is always in scope for fulfillment of dependencies.
        // - Following the above, sibling dependencies are disallowed as well.
        // - Edges are ordered and evaluation is performed from low to high ordering, unless other rules require reshuffling the edges.

        // todo children pointing to ancestor handling

        // Child edges are ordered, execution rule is low to high in the graph, unless other rules override.
        let child_edges = graph.outgoing_edges(&node);
        let mut direct_children: Vec<(EdgeRef, NodeRef)> = child_edges
            .into_iter()
            .filter_map(|edge| {
                let child_node = graph.edge_target(&edge);

                if graph.is_direct_child(&node, &child_node) {
                    Some((edge, child_node))
                } else {
                    None
                }
            })
            .collect();

        // Find the positions of all result returning graph nodes
        let mut result_positions: Vec<usize> = direct_children
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

        let mut result_scopes: Vec<Vec<(EdgeRef, NodeRef)>> = vec![];

        // Start splitting of at the end to keep indices intact
        result_positions.reverse();
        result_positions.iter().for_each(|pos| {
            let scope = direct_children.split_off(*pos);
            result_scopes.push(scope);
        });

        // Because we split from right to left, everything left in the direct_children
        // doesn't belong into results, and is executed before all result scopes.
        let mut expressions: Vec<Expression> = direct_children
            .into_iter()
            .map(|(edge, node)| Self::build_expression(graph, node, Some(edge)))
            .collect::<QueryExecutionResult<Vec<Expression>>>()?;

        if result_scopes.len() > 0 {
            let result_exp = Self::fold_result_scopes(graph, vec![], result_scopes)?;
            expressions.push(result_exp);
        }

        // --- children end ---

        let is_result = graph.is_result_node(&node);
        let node_id = node.id();
        let query: Query = graph.pluck_node(node).try_into()?;
        let exp = Self::query_expression(graph, parent_edge, query);

        if expressions.is_empty() {
            Ok(exp)
        } else {
            let node_binding_name = format!("{}", node_id);

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
                    exp,
                }],
                expressions,
            })
        }

        // --------------------------

        // Split the children into a set of siblings to be executed before the result and one with/after the result subgraph.
        // let result_position = child_edges.iter().position(|edge| {
        //     let target = graph.edge_target(edge);
        //     graph.subgraph_contains_result(&target)
        // });

        // let (before, after) = match result_position {
        //     Some(pos) => (child_edges.split_off(pos), child_edges),
        //     None => (child_edges, vec![]),
        // };

        // let mut before_result: Vec<_> = before
        //     .into_iter()
        //     .map(|child_edge| Self::build_expression(graph, graph.edge_target(&child_edge), Some(child_edge)))
        //     .collect();

        // let mut after_result: Vec<_> = after
        //     .into_iter()
        //     .map(|child_edge| Self::build_expression(graph, graph.edge_target(&child_edge), Some(child_edge)))
        //     .collect();

        // Append a let binding to bind the result to a variable and allow it to be returned last to propagate back up the call chain.
        // By the virtue of the split, the subgraph root node that contains the result is first in the `after_result` vector.
        // If `after_vector` contains only the expression of the result graph, just append it to the end - no additional bindings
        // are required.
        // let mut expressions = if after_result.len() > 1 {
        //     let head = after_result.split_off(0).pop().unwrap();

        //     after_result.push(Expression::Get {
        //         binding_name: "result".to_owned(),
        //     });

        //     before_result.push(Expression::Let {
        //         bindings: vec![Binding {
        //             name: "result".to_owned(),
        //             exp: head,
        //         }],
        //         expressions: after_result,
        //     });

        //     before_result
        // } else {
        //     before_result.append(&mut after_result);
        //     before_result
        // };

        // // Writes before reads
        // let (write_edges, read_edges): (Vec<_>, Vec<_>) = child_edges.into_iter().partition(|child_edge| match &*graph
        //     .edge_content(&child_edge)
        // {
        //     QueryGraphDependency::Write(_, _) => true,
        //     QueryGraphDependency::Read(_) => false,
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

        // if expressions.is_empty() {
        //     exp
        // } else {
        //     // Add a final statement to the evaluation if the current node has child nodes and is supposed to be the
        //     // final result, to make sure it propagates upwards.
        //     if is_result {
        //         expressions.push(Expression::Get {
        //             binding_name: "parent".to_owned(),
        //         });
        //     };

        //     Expression::Let {
        //         bindings: vec![Binding {
        //             name: "parent".to_owned(),
        //             exp,
        //         }],
        //         expressions,
        //     }
        // }
    }

    // All result scopes are nested into each other to allow a final select at the end,
    // resulting in a single expression.
    fn fold_result_scopes(
        graph: &mut QueryGraph,
        mut result_binding_names: Vec<String>,
        mut result_scopes: Vec<Vec<(EdgeRef, NodeRef)>>,
    ) -> QueryExecutionResult<Expression> {
        let current_scope = result_scopes.split_off(0).pop().unwrap();
        let result_binding_name = current_scope.first().unwrap().1.id();
        let has_sub_scopes = result_scopes.len() > 0; // todo necessary?
        let has_parent_scopes = result_binding_names.len() > 0;

        result_binding_names.push(result_binding_name.clone());

        let mut expressions: Vec<Expression> = current_scope
            .into_iter()
            .map(|(edge, node)| Self::build_expression(graph, node, Some(edge)))
            .collect::<QueryExecutionResult<Vec<Expression>>>()?;

        // The first expression of every result scope is the one returning a potential result.
        let head = expressions.split_off(0).pop().unwrap();

        // If there are more scopes, build the nested ones.
        let last_expression = if has_sub_scopes {
            Self::fold_result_scopes(graph, result_binding_names, result_scopes)
        // expressions.push(sub_scope);
        } else {
            // Else, build the final select if necessary
            Ok(Expression::GetFirstNonEmpty {
                binding_names: result_binding_names,
            })
        }?;

        // When to bind the result expression to a Let:
        // - Result node has child expressions.
        // - Current scope has more subscopes.
        // - Current scope has parent scopes.
        if expressions.len() > 0 || has_sub_scopes || has_parent_scopes {
            expressions.push(last_expression);

            Ok(Expression::Let {
                bindings: vec![Binding {
                    name: result_binding_name,
                    exp: head,
                }],
                expressions, // last_expression
            })
        } else {
            Ok(head)
        }
    }

    fn query_expression(graph: &mut QueryGraph, parent_edge: Option<EdgeRef>, query: Query) -> Expression {
        match (parent_edge, query) {
            (None, query) => Expression::Query { query },

            (Some(child_edge), query) => {
                // let parent_binding_name = graph.edge_source(&child_edge).id();
                // match graph.pluck_edge(child_edge) {
                //     QueryGraphDependency::ParentId(f) => Expression::Func {
                //         func: Box::new(move |env: Env| {
                //             let parent_result = env.get(&parent_binding_name).unwrap();
                //             let parent_id = parent_result.as_id();
                //             let query = f(query, parent_id);

                //             Expression::Query { query }
                //         }),
                //     },
                //     QueryGraphDependency::ExecutionOrder => Expression::Query { query },
                //     QueryGraphDependency::Conditional(f) => Expression::If {
                //         func: Box::new(move |env: Env| {
                //             let parent_id = env.get(&parent_binding_name).map(|pid| pid.as_id());
                //             f(parent_id)
                //         }),
                //     },
                //     _ => unreachable!(),
                // }

                unimplemented!()
            }
        }
    }
}
