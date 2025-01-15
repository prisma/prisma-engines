mod query;

use query::translate_query;
use query_builder::QueryBuilder;
use thiserror::Error;

use crate::{EdgeRef, Node, NodeRef, Query, QueryGraph};

use super::expression::{Binding, Expression};

#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("node {0} has no content")]
    NodeContentEmpty(String),

    #[error("query builder error: {0}")]
    QueryBuildFailure(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub type TranslateResult<T> = Result<T, TranslateError>;

pub fn translate(mut graph: QueryGraph, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    graph
        .root_nodes()
        .into_iter()
        .map(|node| NodeTranslator::new(&mut graph, node, &[], builder).translate())
        .collect::<TranslateResult<Vec<_>>>()
        .map(Expression::Seq)
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
            _ => unimplemented!(),
        }
    }

    fn translate_query(&mut self) -> TranslateResult<Expression> {
        self.graph.mark_visited(&self.node);

        let query: Query = self
            .graph
            .pluck_node(&self.node)
            .try_into()
            .expect("current node must be query");

        translate_query(query, self.query_builder)
    }

    #[allow(dead_code)]
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
            .map(|(_, node)| {
                let edges = self.graph.incoming_edges(&node);
                NodeTranslator::new(self.graph, node, &edges, self.query_builder).translate()
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Fold result scopes into one expression.
        if !result_subgraphs.is_empty() {
            let result_exp = self.fold_result_scopes(result_subgraphs)?;
            expressions.push(result_exp);
        }

        Ok(expressions)
    }

    #[allow(dead_code)]
    fn fold_result_scopes(&mut self, result_subgraphs: Vec<(EdgeRef, NodeRef)>) -> TranslateResult<Expression> {
        // if the subgraphs all point to the same result node, we fold them in sequence
        // if not, we can separate them with a getfirstnonempty
        let bindings = result_subgraphs
            .into_iter()
            .map(|(_, node)| {
                let name = node.id();
                let edges = self.graph.incoming_edges(&node);
                let expr = NodeTranslator::new(self.graph, node, &edges, self.query_builder).translate()?;
                Ok(Binding { name, expr })
            })
            .collect::<TranslateResult<Vec<_>>>()?;

        let result_nodes = self.graph.result_nodes();
        let result_binding_names = bindings.iter().map(|b| b.name.clone()).collect::<Vec<_>>();

        if result_nodes.len() == 1 {
            Ok(Expression::Let {
                bindings,
                expr: Box::new(Expression::Get {
                    name: result_binding_names
                        .into_iter()
                        .last()
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
}
