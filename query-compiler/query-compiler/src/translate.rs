mod query;

use super::expression::{Binding, Expression};
use crate::binding;
use crate::data_mapper::map_result_structure;
use crate::expression::EnumsMap;
use crate::result_node::ResultNodeBuilder;
use crate::{Expression::Transaction, selection::projected_placeholder};
use itertools::{Either, Itertools};
use query::{query_guarantees_raw_nested_read_root, translate_query};
use query_builder::QueryBuilder;
use query_core::{
    Computation, EdgeRef, Flow, Node, NodeRef, Query, QueryGraph, QueryGraphBuilderError, QueryGraphDependency,
    QueryGraphError, RowCountSink, RowSink, UpdateManyRecords, WriteQuery,
};
use query_structure::{
    FieldSelection, FieldTypeInformation, IntoFilter, Placeholder, PrismaValue, PrismaValueType, RecordFilter,
    SelectedField, SelectionResult, WriteArgs,
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

enum RootNodes {
    None,
    One(NodeRef),
    Many(Vec<NodeRef>),
}

pub fn translate(mut graph: QueryGraph, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    // Must collect the root nodes first, because the following iteration is mutating the graph
    let root_nodes = {
        let mut nodes = graph.root_nodes();
        match (nodes.next(), nodes.next()) {
            (None, _) => RootNodes::None,
            (Some(node), None) => RootNodes::One(node),
            (Some(first), Some(second)) => {
                let mut roots = Vec::with_capacity(2 + nodes.size_hint().0);
                roots.push(first);
                roots.push(second);
                roots.extend(nodes);
                RootNodes::Many(roots)
            }
        }
    };

    let skip_result_mapping = root_guarantees_raw_nested_read(&graph, &root_nodes);
    let mut enums = EnumsMap::new();
    let mut result_node_builder = ResultNodeBuilder::new(&mut enums);
    let structure = if skip_result_mapping {
        None
    } else {
        map_result_structure(&graph, &mut result_node_builder)
    };
    let result_reachability = ResultReachability::new(&graph);
    graph.reserve_visited_capacity();

    let root = match root_nodes {
        RootNodes::None => Expression::Seq(Vec::new()),
        RootNodes::One(node) => {
            NodeTranslator::new(&mut graph, node, &[], builder, &result_reachability).translate()?
        }
        RootNodes::Many(nodes) => nodes
            .into_iter()
            .map(|node| NodeTranslator::new(&mut graph, node, &[], builder, &result_reachability).translate())
            .collect::<TranslateResult<Vec<_>>>()
            .map(Expression::Seq)?,
    };

    if skip_result_mapping && !matches!(root, Expression::RawNestedRead { .. }) {
        return Err(TranslateError::GraphBuildError(
            QueryGraphBuilderError::QueryGraphError(QueryGraphError::InvariantViolation(
                "raw nested read preflight did not produce a raw nested read".into(),
            )),
        ));
    }

    let mut root = if let Some(structure) = structure {
        if matches!(root, Expression::RawNestedRead { .. }) {
            root
        } else {
            Expression::DataMap {
                expr: Box::new(root),
                structure,
                enums,
            }
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

fn root_guarantees_raw_nested_read(graph: &QueryGraph, root_nodes: &RootNodes) -> bool {
    let RootNodes::One(node) = root_nodes else {
        return false;
    };

    let Some(Node::Query(query)) = graph.node_content(node) else {
        return false;
    };

    query_guarantees_raw_nested_read_root(query)
}

struct ResultReachability {
    can_reach_result: Vec<bool>,
}

impl ResultReachability {
    fn new(graph: &QueryGraph) -> Self {
        let mut can_reach_result = Vec::new();
        let mut pending = graph.result_nodes().collect::<Vec<_>>();

        for node in &pending {
            Self::mark_reachable(&mut can_reach_result, *node);
        }

        while let Some(node) = pending.pop() {
            for parent in graph.parent_nodes(&node) {
                if Self::mark_reachable(&mut can_reach_result, parent) {
                    pending.push(parent);
                }
            }
        }

        Self { can_reach_result }
    }

    fn contains(&self, node: &NodeRef) -> bool {
        self.can_reach_result.get(node.index()).copied().unwrap_or(false)
    }

    fn mark_reachable(can_reach_result: &mut Vec<bool>, node: NodeRef) -> bool {
        let index = node.index();
        if index >= can_reach_result.len() {
            can_reach_result.resize(index + 1, false);
        }

        if can_reach_result[index] {
            false
        } else {
            can_reach_result[index] = true;
            true
        }
    }
}

struct NodeTranslator<'a, 'b> {
    graph: &'a mut QueryGraph,
    node: NodeRef,
    parent_edges: &'b [EdgeRef],
    query_builder: &'b dyn QueryBuilder,
    result_reachability: &'b ResultReachability,
}

impl<'a, 'b> NodeTranslator<'a, 'b> {
    fn new(
        graph: &'a mut QueryGraph,
        node: NodeRef,
        parent_edges: &'b [EdgeRef],
        query_builder: &'b dyn QueryBuilder,
        result_reachability: &'b ResultReachability,
    ) -> Self {
        Self {
            graph,
            node,
            parent_edges,
            query_builder,
            result_reachability,
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
            Node::Flow(Flow::Return(_)) => self.translate_return(false),
            Node::Flow(Flow::ReturnPreservingResult(_)) => self.translate_return(true),
            Node::Computation(Computation::DiffLeftToRight(_)) => self.translate_diff_left_to_right(),
            Node::Computation(Computation::DiffRightToLeft(_)) => self.translate_diff_right_to_left(),
            Node::Computation(Computation::RequiredOneToManySet(_)) => self.translate_required_one_to_many_set(),
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

    fn wrap_children_preserving_expr(&self, expr: Expression, mut children: Vec<Expression>) -> Expression {
        if children.is_empty() {
            return expr;
        }

        let result_name = binding::node_result(self.node);
        children.push(Expression::Get {
            name: result_name.clone(),
        });

        Expression::Let {
            bindings: vec![Binding::new(result_name, expr)],
            expr: Expression::Seq(children).into(),
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

        let then_returns_condition = match self.graph.node_content(&self.node) {
            Some(Node::Flow(Flow::If {
                then_returns_condition, ..
            })) => *then_returns_condition,
            _ => false,
        };

        let then_expr = match then_node {
            Some(node) => Some(self.process_child_with_dependencies(node)?),
            None if then_returns_condition => None,
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

        let Node::Flow(Flow::If { rule, data, .. }) = node else {
            panic!("current node must be Flow::If");
        };
        let placeholder = projected_placeholder(data, "If")?;
        let then_expr = then_expr.unwrap_or_else(|| Expression::Get {
            name: placeholder.name.clone(),
        });

        let expr = Expression::If {
            value: Expression::Get { name: placeholder.name }.into(),
            rule,
            then: then_expr.into(),
            r#else: else_expr.into(),
        };

        Ok(self.wrap_children_with_expr(expr, children))
    }

    fn translate_return(&mut self, preserve_result_after_children: bool) -> TranslateResult<Expression> {
        let children = self.translate_children()?;

        let node = self.graph.pluck_node(&self.node);
        let node = self.transform_node(node)?;

        let Node::Flow(Flow::Return(data) | Flow::ReturnPreservingResult(data)) = node else {
            panic!("current node must be Flow::Return");
        };
        let placeholder = projected_placeholder(data, "Return")?;

        let expr = Expression::Get { name: placeholder.name };

        if preserve_result_after_children {
            Ok(self.wrap_children_preserving_expr(expr, children))
        } else {
            Ok(self.wrap_children_with_expr(expr, children))
        }
    }

    fn translate_diff_left_to_right(&mut self) -> TranslateResult<Expression> {
        let children = self.translate_children()?;

        let node = self.graph.pluck_node(&self.node);
        let node = self.transform_node(node)?;

        let Node::Computation(Computation::DiffLeftToRight(diff)) = node else {
            panic!("current node must be Computation::DiffLeftToRight");
        };

        let from = projected_placeholder(diff.left, "DiffLeftToRight.left")?;
        let to = projected_placeholder(diff.right, "DiffLeftToRight.right")?;

        let expr = Expression::Diff {
            from: Expression::Get { name: from.name }.into(),
            to: Expression::Get { name: to.name }.into(),
            fields: diff.fields.db_names().collect(),
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

        let from = projected_placeholder(diff.right, "DiffRightToLeft.right")?;
        let to = projected_placeholder(diff.left, "DiffRightToLeft.left")?;

        let expr = Expression::Diff {
            from: Expression::Get { name: from.name }.into(),
            to: Expression::Get { name: to.name }.into(),
            fields: diff.fields.db_names().collect(),
        };

        Ok(self.wrap_children_with_expr(expr, children))
    }

    fn translate_required_one_to_many_set(&mut self) -> TranslateResult<Expression> {
        let children = self.translate_children()?;

        let node = self.graph.pluck_node(&self.node);
        let node = self.transform_node(node)?;

        let Node::Computation(Computation::RequiredOneToManySet(set)) = node else {
            panic!("current node must be Computation::RequiredOneToManySet");
        };

        let old_children = projected_placeholder(set.old_children, "RequiredOneToManySet.old_children")?;
        let new_children = projected_placeholder(set.new_children, "RequiredOneToManySet.new_children")?;
        let left_diff_name = binding::node_result(self.node);

        let selector = SelectionResult::new(
            set.fields
                .selections()
                .map(|field| {
                    (
                        field.clone(),
                        PrismaValue::Placeholder(Placeholder::new(
                            binding::projected_dependency(self.node, field),
                            selected_field_placeholder_type(field, true),
                        )),
                    )
                })
                .collect(),
        );

        let mut update_args = WriteArgs::from_result(
            SelectionResult::new(
                set.parent_link
                    .selections()
                    .zip(set.child_link.selections())
                    .map(|(parent_field, child_field)| {
                        (
                            child_field.clone(),
                            PrismaValue::Placeholder(Placeholder::new(
                                binding::projected_dependency(set.parent_node, parent_field),
                                selected_field_placeholder_type(parent_field, false),
                            )),
                        )
                    })
                    .collect(),
            ),
            set.request_now,
        );
        update_args.update_datetimes(&set.child_model);

        let update = UpdateManyRecords {
            name: String::new(),
            model: set.child_model,
            record_filter: RecordFilter::from(vec![selector]),
            args: update_args,
            selected_fields: None,
            limit: None,
        };

        let update_expr = translate_query(Query::Write(WriteQuery::UpdateManyRecords(update)), self.query_builder)?;
        let parent_validation = Expression::validate_expectation(
            &set.parent_expectation,
            Expression::Get {
                name: binding::node_result(set.parent_node),
            },
        );

        let relation_validation = Expression::validate_expectation(
            &set.relation_expectation,
            Expression::Diff {
                from: Expression::Get {
                    name: old_children.name.clone(),
                }
                .into(),
                to: Expression::Get {
                    name: new_children.name.clone(),
                }
                .into(),
                fields: set.fields.db_names().collect(),
            },
        );

        let expr = Expression::Let {
            bindings: vec![Binding::new(
                left_diff_name.clone(),
                Expression::Diff {
                    from: Expression::Get {
                        name: new_children.name.clone(),
                    }
                    .into(),
                    to: Expression::Get {
                        name: old_children.name.clone(),
                    }
                    .into(),
                    fields: set.fields.db_names().collect(),
                },
            )],
            expr: Expression::Seq(vec![
                Expression::If {
                    value: Expression::Get {
                        name: left_diff_name.clone(),
                    }
                    .into(),
                    rule: query_core::DataRule::RowCountNeq(0),
                    then: Expression::Let {
                        bindings: set
                            .fields
                            .selections()
                            .map(|field| {
                                Binding::new(
                                    binding::projected_dependency(self.node, field),
                                    Expression::MapField {
                                        field: field.db_name().into(),
                                        records: Expression::Get {
                                            name: left_diff_name.clone(),
                                        }
                                        .into(),
                                    },
                                )
                            })
                            .collect(),
                        expr: Expression::Seq(vec![parent_validation, update_expr]).into(),
                    }
                    .into(),
                    r#else: Expression::Unit.into(),
                },
                relation_validation,
            ])
            .into(),
        };

        Ok(self.wrap_children_with_expr(expr, children))
    }

    fn transform_node(&mut self, mut node: Node) -> TranslateResult<Node> {
        for edge in self.parent_edges {
            match self.graph.take_edge(edge) {
                Some(QueryGraphDependency::ProjectedDataDependency(projected_selection, sink, _)) => match sink {
                    RowSink::All(field) | RowSink::ExactlyOne(field) | RowSink::AtMostOne(field) => {
                        let fields = self.process_edge_selections(edge, &node, projected_selection);
                        *field.node_input_field(&mut node) = vec![SelectionResult::new(fields)];
                    }
                    RowSink::Single(field) => {
                        let fields = self.process_edge_selections(edge, &node, projected_selection);
                        *field.node_input_field(&mut node) = Some(SelectionResult::new(fields));
                    }
                    RowSink::ProjectedPlaceholder(field) => {
                        *field.node_input_field(&mut node) =
                            Some(self.process_edge_placeholder(edge, &node, projected_selection)?);
                    }
                    RowSink::AllFilter(field) | RowSink::ExactlyOneFilter(field) => {
                        let fields = self.process_edge_selections(edge, &node, projected_selection);
                        *field.node_input_field(&mut node) = SelectionResult::new(fields).filter();
                    }
                    RowSink::ExactlyOneWriteArgs(write_arg_selection, field) => {
                        let fields = self.process_edge_selections(edge, &node, projected_selection);
                        let result = SelectionResult::new(fields);
                        let model = node.as_query().map(Query::model);
                        let args = field.node_input_field(&mut node);
                        for arg in args {
                            arg.inject(write_arg_selection.assimilate(result.clone()).map_err(|err| {
                                TranslateError::GraphBuildError(QueryGraphBuilderError::DomainError(err))
                            })?);
                            if let Some(model) = &model {
                                arg.update_datetimes(model);
                            }
                        }
                    }
                    RowSink::Discard => {}
                },

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
                if self.result_reachability.contains(child_node) {
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
        if let [(_, node)] = &result_subgraphs[..] {
            return self.process_child_with_dependencies(*node);
        }

        // if the subgraphs all point to the same result node, we fold them in sequence
        // if not, we can separate them with a getfirstnonempty
        let bindings = result_subgraphs
            .into_iter()
            .map(|(_, node)| {
                let expr = self.process_child_with_dependencies(node)?;
                Ok(Binding::new(binding::node_result(node), expr))
            })
            .collect::<TranslateResult<Vec<_>>>()?;

        let has_single_result_node = self.graph.result_nodes().take(2).count() == 1;

        if has_single_result_node {
            let result_binding_name = bindings.last().expect("no binding for result node").name.clone();
            Ok(Expression::Let {
                bindings,
                expr: Box::new(Expression::Get {
                    name: result_binding_name,
                }),
            })
        } else {
            let result_binding_names = bindings.iter().map(|b| b.name.clone()).collect::<Vec<_>>();
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
        let incoming_edges = self.graph.incoming_edges(&node);

        let validations = incoming_edges
            .iter()
            .filter_map(|edge| {
                let Some(QueryGraphDependency::DataDependency(RowCountSink::Discard, expectation)) =
                    self.graph.edge_content(edge)
                else {
                    return None;
                };
                let mut expr = Expression::Get {
                    name: binding::node_result(self.graph.edge_source(edge)),
                };
                if let Some(expectation) = expectation {
                    expr = Expression::validate_expectation(expectation, expr);
                }
                Some(expr)
            })
            .collect_vec();

        let bindings = incoming_edges
            .iter()
            .flat_map(|edge| {
                let edge_content = self.graph.edge_content(edge);
                let Some(QueryGraphDependency::ProjectedDataDependency(selection, sink, expectation)) = edge_content
                else {
                    return Either::Left(std::iter::empty());
                };

                let requires_unique = sink.is_unique();

                let source = self.graph.edge_source(edge);

                let needs_parent_binding = expectation.is_some() || requires_unique;
                let parent_binding = needs_parent_binding.then(|| {
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

                    Binding::new(source.id(), expr)
                });

                let parent_bindings = parent_binding.into_iter();

                if create_field_bindings {
                    Either::Right(Either::Left(parent_bindings.chain(selection.selections().map(
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
                    Either::Right(Either::Right(parent_bindings))
                }
            })
            .collect::<Vec<_>>();

        // translate plucks the edges coming into node, we need to avoid accessing it afterwards
        let expr = NodeTranslator::new(
            self.graph,
            node,
            &incoming_edges,
            self.query_builder,
            self.result_reachability,
        )
        .translate()?;

        if validations.is_empty() {
            return Ok(if bindings.is_empty() {
                expr
            } else {
                Expression::Let {
                    bindings,
                    expr: Box::new(expr),
                }
            });
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

    fn process_edge_placeholder(
        &mut self,
        edge: &EdgeRef,
        node: &Node,
        selection: FieldSelection,
    ) -> TranslateResult<Placeholder> {
        let bindings_refer_to_fields = matches!(node, Node::Query(_));
        let binding_is_unique = matches!(node, Node::Query(q) if q.is_unique());
        let field = selection.selections().next().ok_or_else(|| {
            TranslateError::GraphBuildError(QueryGraphBuilderError::QueryGraphError(
                QueryGraphError::InvariantViolation("Projected placeholder sink requires at least one field".into()),
            ))
        })?;

        let r#type = selected_field_placeholder_type(field, !binding_is_unique);

        Ok(Placeholder {
            name: if bindings_refer_to_fields {
                binding::projected_dependency(self.graph.edge_source(edge), field)
            } else {
                binding::node_result(self.graph.edge_source(edge))
            },
            r#type,
        })
    }
}

fn selected_field_placeholder_type(field: &SelectedField, list: bool) -> PrismaValueType {
    let r#type = field
        .type_info()
        .as_ref()
        .map(FieldTypeInformation::to_prisma_type)
        .unwrap_or(PrismaValueType::Any);

    if list {
        PrismaValueType::List(r#type.into())
    } else {
        r#type
    }
}
