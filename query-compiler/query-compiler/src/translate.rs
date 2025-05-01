mod query;

use super::expression::{Binding, Expression};
use crate::Expression::Transaction;
use crate::data_mapper::map_result_structure;
use itertools::{Either, Itertools};
use query::translate_query;
use query_builder::QueryBuilder;
use query_core::{EdgeRef, Node, NodeRef, Query, QueryGraph, QueryGraphBuilderError, QueryGraphDependency};
use query_structure::{PrismaValue, PrismaValueType, SelectedField, SelectionResult};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("node {0} has no content")]
    NodeContentEmpty(String),

    #[error("query builder error: {0}")]
    QueryBuildFailure(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("query graph build error: {0}")]
    GraphBuildError(#[from] QueryGraphBuilderError),
}

pub type TranslateResult<T> = Result<T, TranslateError>;

pub fn translate(mut graph: QueryGraph, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    let structure = map_result_structure(&graph);

    // Must collect the root nodes first, because the following iteration is mutating the graph
    let root_nodes: Vec<NodeRef> = graph.root_nodes().collect();

    let root = root_nodes
        .into_iter()
        .map(|node| NodeTranslator::new(&mut graph, node, &[], builder).translate())
        .collect::<TranslateResult<Vec<_>>>()
        .map(Expression::Seq)?;

    let root = if let Some(structure) = structure {
        Expression::DataMap {
            expr: Box::new(root),
            structure,
        }
    } else {
        root
    };

    if graph.needs_transaction() {
        return Ok(Transaction(Box::new(root)));
    }

    Ok(root)
}

struct NodeTranslator<'a, 'b> {
    graph: &'a mut QueryGraph,
    node: NodeRef,
    #[allow(dead_code)]
    parent_edges: &'b [EdgeRef],
    query_builder: &'b dyn QueryBuilder,
}

impl<'a, 'b> NodeTranslator<'a, 'b> {
    fn new(
        graph: &'a mut QueryGraph,
        node: NodeRef,
        parent_edges: &'b [EdgeRef],
        query_builder: &'b dyn QueryBuilder,
    ) -> Self {
        Self {
            graph,
            node,
            parent_edges,
            query_builder,
        }
    }

    fn translate(&mut self) -> TranslateResult<Expression> {
        let node = self
            .graph
            .node_content(&self.node)
            .ok_or_else(|| TranslateError::NodeContentEmpty(self.node.id()))?;

        match node {
            Node::Query(_) => self.translate_query(),
            // might be worth having Expression::Unit for this?
            Node::Empty => Ok(Expression::Seq(vec![])),
            n => unimplemented!("{:?}", std::mem::discriminant(n)),
        }
    }

    fn translate_query(&mut self) -> TranslateResult<Expression> {
        self.graph.mark_visited(&self.node);

        // Don't recurse into children if the current node is already a result node.
        let children = if !self.graph.is_result_node(&self.node) {
            self.process_children()?
        } else {
            Vec::new()
        };

        let mut node = self.graph.pluck_node(&self.node);

        for edge in self.parent_edges {
            match self.graph.pluck_edge(edge) {
                QueryGraphDependency::ExecutionOrder => {}
                QueryGraphDependency::ProjectedDataDependency(selection, f, _) => {
                    let fields = selection
                        .selections()
                        .map(|field| {
                            (
                                field.clone(),
                                PrismaValue::Placeholder {
                                    name: generate_projected_dependency_name(self.graph.edge_source(edge), field),
                                    r#type: PrismaValueType::Any,
                                },
                            )
                        })
                        .collect_vec();

                    // TODO: there are cases where we look at the number of results in some
                    // dependencies, these won't work with the current implementation and will
                    // need to be re-implemented
                    node = f(node, vec![SelectionResult::new(fields)])?;
                }
                // TODO: implement data dependencies and if/else
                QueryGraphDependency::DataDependency(_) => todo!(),
                QueryGraphDependency::Then => todo!(),
                QueryGraphDependency::Else => todo!(),
            };
        }

        let query: Query = node.try_into().expect("current node must be query");
        let expr = translate_query(query, self.query_builder)?;

        if !children.is_empty() {
            Ok(Expression::Let {
                bindings: vec![Binding::new(self.node.id(), expr)],
                expr: Box::new(Expression::Seq(children)),
            })
        } else {
            Ok(expr)
        }
    }

    fn process_children(&mut self) -> TranslateResult<Vec<Expression>> {
        let mut child_pairs = self.graph.direct_child_pairs(&self.node);

        // Find the positions of all result returning graph nodes.
        let mut result_positions = child_pairs
            .iter()
            .enumerate()
            .filter_map(|(idx, (_, child_node))| {
                if self.graph.subgraph_contains_result(child_node) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Start removing the highest indices first to not invalidate subsequent removals.
        result_positions.sort_unstable();
        result_positions.reverse();

        let result_subgraphs = result_positions
            .into_iter()
            .map(|pos| child_pairs.remove(pos))
            .collect::<Vec<_>>();

        // Because we split from right to left, everything remaining in `child_pairs`
        // doesn't belong into results, and is executed before all result scopes.
        let mut expressions: Vec<Expression> = child_pairs
            .into_iter()
            .map(|(_, node)| self.process_child_with_dependencies(node))
            .collect::<Result<Vec<_>, _>>()?;

        // Fold result scopes into one expression.
        if !result_subgraphs.is_empty() {
            let result_exp = self.fold_result_scopes(result_subgraphs)?;
            expressions.push(result_exp);
        }

        Ok(expressions)
    }

    fn fold_result_scopes(&mut self, result_subgraphs: Vec<(EdgeRef, NodeRef)>) -> TranslateResult<Expression> {
        // if the subgraphs all point to the same result node, we fold them in sequence
        // if not, we can separate them with a getfirstnonempty
        let bindings = result_subgraphs
            .into_iter()
            .map(|(_, node)| {
                let name = node.id();
                let expr = self.process_child_with_dependencies(node)?;
                Ok(Binding { name, expr })
            })
            .collect::<TranslateResult<Vec<_>>>()?;

        let result_nodes: Vec<NodeRef> = self.graph.result_nodes().collect();
        let result_binding_names = bindings.iter().map(|b| b.name.clone()).collect::<Vec<_>>();

        if result_nodes.len() == 1 {
            Ok(Expression::Let {
                bindings,
                expr: Box::new(Expression::Get {
                    name: result_binding_names
                        .into_iter()
                        .next_back()
                        .expect("no binding for result node"),
                }),
            })
        } else {
            Ok(Expression::Let {
                bindings,
                expr: Box::new(Expression::GetFirstNonEmpty {
                    names: result_binding_names,
                }),
            })
        }
    }

    fn process_child_with_dependencies(&mut self, node: NodeRef) -> TranslateResult<Expression> {
        let bindings = self
            .graph
            .incoming_edges(&node)
            .into_iter()
            .flat_map(|edge| {
                let Some(QueryGraphDependency::ProjectedDataDependency(selection, _, _)) =
                    self.graph.edge_content(&edge)
                else {
                    return Either::Left(std::iter::empty());
                };

                let source = self.graph.edge_source(&edge);

                Either::Right(selection.selections().map(move |field| {
                    Binding::new(
                        generate_projected_dependency_name(source, field),
                        Expression::MapField {
                            field: field.prisma_name().into_owned(),
                            records: Box::new(Expression::Get { name: source.id() }),
                        },
                    )
                }))
            })
            .collect::<Vec<_>>();

        // translate plucks the edges coming into node, we need to avoid accessing it afterwards
        let edges = self.graph.incoming_edges(&node);
        let expr = NodeTranslator::new(self.graph, node, &edges, self.query_builder).translate()?;

        // we insert a MapField expression if the edge was a projected data dependency
        if !bindings.is_empty() {
            Ok(Expression::Let {
                bindings,
                expr: Box::new(expr),
            })
        } else {
            Ok(expr)
        }
    }
}

fn generate_projected_dependency_name(source: NodeRef, field: &SelectedField) -> String {
    format!("{}${}", source.id(), field.prisma_name())
}
