mod query;

use super::expression::{Binding, Expression};
use crate::binding;
use crate::data_mapper::map_result_structure;
use crate::expression::EnumsMap;
use crate::result_node::ResultNodeBuilder;
use crate::{Expression::Transaction, selection::SelectionResults};
use itertools::{Either, Itertools};
use query::translate_query;
use query_builder::QueryBuilder;
use query_core::{
    Computation, EdgeRef, Flow, Node, NodeRef, Query, QueryGraph, QueryGraphBuilderError, QueryGraphDependency,
    QueryGraphError, RowCountSink, RowSink,
};
use query_structure::{
    FieldSelection, FieldTypeInformation, IntoFilter, Placeholder, PrismaValue, PrismaValueType, SelectedField,
    SelectionResult,
};
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
    let mut enums = EnumsMap::new();
    let mut result_node_builder = ResultNodeBuilder::new(&mut enums);
    let structure = map_result_structure(&graph, &mut result_node_builder);

    // Must collect the root nodes first, because the following iteration is mutating the graph
    let root_nodes: Vec<NodeRef> = graph.root_nodes().collect();

    let root = root_nodes
        .into_iter()
        .map(|node| NodeTranslator::new(&mut graph, node, &[], builder).translate())
        .collect::<TranslateResult<Vec<_>>>()
        .map(Expression::Seq)?;

    let mut root = if let Some(structure) = structure {
        Expression::DataMap {
            expr: Box::new(root),
            structure,
            enums,
        }
    } else {
        root
    };

    root.simplify();
    if graph.needs_transaction() {
        return Ok(Transaction(Box::new(root)));
    }
    Ok(root)
}

struct NodeTranslator<'a, 'b> {
    graph: &'a mut QueryGraph,
    node: NodeRef,
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
        self.graph.mark_visited(&self.node);
        let node = self
            .graph
            .node_content(&self.node)
            .ok_or_else(|| TranslateError::NodeContentEmpty(self.node.id()))?;

        match node {
            Node::Query(_) => self.translate_query(),
            Node::Empty => {
                let children = self.translate_children()?;
                Ok(if children.is_empty() {
                    Expression::Unit
                } else {
                    Expression::Seq(children)
                })
            }
            Node::Flow(Flow::If { .. }) => self.translate_if(),
            Node::Flow(Flow::Return(_)) => self.translate_return(),
            Node::Computation(Computation::DiffLeftToRight(_)) => self.translate_diff_left_to_right(),
            Node::Computation(Computation::DiffRightToLeft(_)) => self.translate_diff_right_to_left(),
        }
    }

    fn translate_children(&mut self) -> TranslateResult<Vec<Expression>> {
        let mut children = self.process_children()?;
        if self.graph.is_result_node(&self.node) {
            children.push(Expression::Get {
                name: binding::node_result(self.node),
            });
        }
        Ok(children)
    }

    fn wrap_children_with_expr(&self, expr: Expression, children: Vec<Expression>) -> Expression {
        if children.is_empty() {
            return expr;
        }
        Expression::Let {
            bindings: vec![Binding::new(binding::node_result(self.node), expr)],
            expr: if children.len() == 1 {
                children.into_iter().next().unwrap().into()
            } else {
                Expression::Seq(children).into()
            },
        }
    }

    fn translate_query(&mut self) -> TranslateResult<Expression> {
        let children = self.translate_children()?;

        let node = self.graph.pluck_node(&self.node);
        let node = self.transform_node(node)?;

        let query: Query = node.try_into().expect("current node must be query");
        let expr = translate_query(query, self.query_builder)?;

        Ok(self.wrap_children_with_expr(expr, children))
    }

    fn translate_if(&mut self) -> TranslateResult<Expression> {
        let mut then_node = None;
        let mut else_node = None;

        for (edge, node) in self.graph.direct_child_pairs(&self.node) {
            match self.graph.edge_content(&edge) {
                Some(QueryGraphDependency::Then) => {
                    if then_node.is_some() {
                        return Err(TranslateError::GraphBuildError(
                            QueryGraphBuilderError::QueryGraphError(QueryGraphError::InvariantViolation(
                                "Multiple Then edges in the If node".into(),
                            )),
                        ));
                    }
                    self.graph.pluck_edge(&edge);
                    then_node = Some(node);
                }
                Some(QueryGraphDependency::Else) => {
                    if else_node.is_some() {
                        return Err(TranslateError::GraphBuildError(
                            QueryGraphBuilderError::QueryGraphError(QueryGraphError::InvariantViolation(
                                "Multiple Else edges in the If node".into(),
                            )),
                        ));
                    }
                    self.graph.pluck_edge(&edge);
                    else_node = Some(node);
                }
                _ => {}
            }
        }

        let then_expr = match then_node {
            Some(node) => self.process_child_with_dependencies(node)?,
            None => {
                return Err(TranslateError::GraphBuildError(
                    QueryGraphBuilderError::QueryGraphError(QueryGraphError::InvariantViolation(
                        "Missing Then edge in the If node".into(),
                    )),
                ));
            }
        };

        let else_expr = match else_node {
            Some(node) => self.process_child_with_dependencies(node)?,
            None => Expression::Unit,
        };

        let children = self.translate_children()?;

        let node = self.graph.pluck_node(&self.node);
        let node = self.transform_node(node)?;

        let Node::Flow(Flow::If { rule, data }) = node else {
            panic!("current node must be Flow::If");
        };

        let expr = Expression::If {
            value: Expression::Get {
                name: SelectionResults::new(data).into_placeholder()?.name,
            }
            .into(),
            rule,
            then: then_expr.into(),
            r#else: else_expr.into(),
        };

        Ok(self.wrap_children_with_expr(expr, children))
    }

    fn translate_return(&mut self) -> TranslateResult<Expression> {
        let children = self.translate_children()?;

        let node = self.graph.pluck_node(&self.node);
        let node = self.transform_node(node)?;

        let Node::Flow(Flow::Return(data)) = node else {
            panic!("current node must be Flow::Return");
        };

        let expr = Expression::Get {
            name: SelectionResults::new(data).into_placeholder()?.name,
        };

        Ok(self.wrap_children_with_expr(expr, children))
    }

    fn translate_diff_left_to_right(&mut self) -> TranslateResult<Expression> {
        let children = self.translate_children()?;

        let node = self.graph.pluck_node(&self.node);
        let node = self.transform_node(node)?;

        let Node::Computation(Computation::DiffLeftToRight(diff)) = node else {
            panic!("current node must be Computation::DiffLeftToRight");
        };

        let expr = Expression::Diff {
            from: Expression::Get {
                name: SelectionResults::new(diff.left).into_placeholder()?.name,
            }
            .into(),
            to: Expression::Get {
                name: SelectionResults::new(diff.right).into_placeholder()?.name,
            }
            .into(),
        };

        Ok(self.wrap_children_with_expr(expr, children))
    }

    fn translate_diff_right_to_left(&mut self) -> TranslateResult<Expression> {
        let children = self.translate_children()?;

        let node = self.graph.pluck_node(&self.node);
        let node = self.transform_node(node)?;

        let Node::Computation(Computation::DiffRightToLeft(diff)) = node else {
            panic!("current node must be Computation::DiffRightToLeft");
        };

        let expr = Expression::Diff {
            from: Expression::Get {
                name: SelectionResults::new(diff.right).into_placeholder()?.name,
            }
            .into(),
            to: Expression::Get {
                name: SelectionResults::new(diff.left).into_placeholder()?.name,
            }
            .into(),
        };

        Ok(self.wrap_children_with_expr(expr, children))
    }

    fn transform_node(&mut self, mut node: Node) -> TranslateResult<Node> {
        for edge in self.parent_edges {
            match self.graph.take_edge(edge) {
                Some(QueryGraphDependency::ProjectedDataDependency(selection, sink, _)) => {
                    let fields = self.process_edge_selections(edge, &node, selection);

                    match sink {
                        RowSink::All(field) | RowSink::ExactlyOne(field) | RowSink::AtMostOne(field) => {
                            *field.node_input_field(&mut node) = vec![SelectionResult::new(fields)];
                        }
                        RowSink::Single(field) => {
                            *field.node_input_field(&mut node) = Some(SelectionResult::new(fields));
                        }
                        RowSink::AllFilter(field) | RowSink::ExactlyOneFilter(field) => {
                            *field.node_input_field(&mut node) = SelectionResult::new(fields).filter();
                        }
                        RowSink::ExactlyOneWriteArgs(selection, field) => {
                            let result = SelectionResult::new(fields);
                            let model = node.as_query().map(Query::model);
                            let args = field.node_input_field(&mut node);
                            for arg in args {
                                arg.inject(selection.assimilate(result.clone()).map_err(|err| {
                                    TranslateError::GraphBuildError(QueryGraphBuilderError::DomainError(err))
                                })?);
                                if let Some(model) = &model {
                                    arg.update_datetimes(model);
                                }
                            }
                        }
                        RowSink::Discard => {}
                    }
                }

                Some(QueryGraphDependency::DataDependency(_, _)) => todo!(),

                Some(QueryGraphDependency::ExecutionOrder)
                | Some(QueryGraphDependency::Then)
                | Some(QueryGraphDependency::Else)
                | None => {}
            };
        }

        Ok(node)
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
                let expr = self.process_child_with_dependencies(node)?;
                Ok(Binding::new(binding::node_result(node), expr))
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
        let create_field_bindings = matches!(self.graph.node_content(&node), Some(Node::Query(_)));

        let validations = self
            .graph
            .incoming_edges(&node)
            .into_iter()
            .filter_map(|edge| {
                let Some(QueryGraphDependency::DataDependency(RowCountSink::Discard, expectation)) =
                    self.graph.edge_content(&edge)
                else {
                    return None;
                };
                let mut expr = Expression::Get {
                    name: binding::node_result(self.graph.edge_source(&edge)),
                };
                if let Some(expectation) = expectation {
                    expr = Expression::validate_expectation(expectation, expr);
                }
                Some(expr)
            })
            .collect_vec();

        let bindings = self
            .graph
            .incoming_edges(&node)
            .into_iter()
            .flat_map(|edge| {
                let edge_content = self.graph.edge_content(&edge);
                let Some(QueryGraphDependency::ProjectedDataDependency(selection, _, expectation)) = edge_content
                else {
                    return Either::Left(std::iter::empty());
                };

                let requires_unique = matches!(
                    edge_content,
                    Some(QueryGraphDependency::ProjectedDataDependency(_, sink, _))
                        if sink.is_unique()
                );

                let source = self.graph.edge_source(&edge);

                let expr = Expression::Get {
                    name: binding::node_result(source),
                };
                let expr = match expectation {
                    Some(expectation) => Expression::validate_expectation(expectation, expr),
                    None => expr,
                };
                let expr = if requires_unique {
                    Expression::Unique(expr.into())
                } else {
                    expr
                };

                let parent_binding = std::iter::once(Binding::new(source.id(), expr));

                if create_field_bindings {
                    Either::Right(Either::Left(parent_binding.chain(selection.selections().map(
                        move |field| {
                            Binding::new(
                                binding::projected_dependency(source, field),
                                Expression::MapField {
                                    field: field.db_name().into(),
                                    records: Expression::Get {
                                        name: binding::node_result(source),
                                    }
                                    .into(),
                                },
                            )
                        },
                    ))))
                } else {
                    Either::Right(Either::Right(parent_binding))
                }
            })
            .filter(|binding| match &binding.expr {
                Expression::Get { name, .. } => name != &binding.name,
                _ => true,
            })
            .collect::<Vec<_>>();

        // translate plucks the edges coming into node, we need to avoid accessing it afterwards
        let edges = self.graph.incoming_edges(&node);
        let expr = NodeTranslator::new(self.graph, node, &edges, self.query_builder).translate()?;

        if bindings.is_empty() && validations.is_empty() {
            return Ok(expr);
        }

        let mut children = validations;
        if !bindings.is_empty() {
            children.push(Expression::Let {
                bindings,
                expr: Box::new(expr),
            })
        } else {
            children.push(expr);
        }
        Ok(Expression::Seq(children))
    }

    fn process_edge_selections(
        &mut self,
        edge: &EdgeRef,
        node: &Node,
        selection: FieldSelection,
    ) -> Vec<(SelectedField, PrismaValue)> {
        let bindings_refer_to_fields = matches!(node, Node::Query(_));
        let binding_is_unique = matches!(node, Node::Query(q) if q.is_unique());

        selection
            .selections()
            .map(|field| {
                let r#type = field
                    .type_info()
                    .as_ref()
                    .map(FieldTypeInformation::to_prisma_type)
                    .unwrap_or(PrismaValueType::Any);
                let r#type = if binding_is_unique {
                    r#type
                } else {
                    PrismaValueType::List(r#type.into())
                };

                (
                    field.clone(),
                    PrismaValue::Placeholder(Placeholder {
                        name: if bindings_refer_to_fields {
                            binding::projected_dependency(self.graph.edge_source(edge), field)
                        } else {
                            binding::node_result(self.graph.edge_source(edge))
                        },
                        r#type,
                    }),
                )
            })
            .collect_vec()
    }
}
