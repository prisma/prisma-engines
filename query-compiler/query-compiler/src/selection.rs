use itertools::Itertools;
use query_core::{QueryGraphBuilderError, QueryGraphError};
use query_structure::{Placeholder, PrismaValue, SelectionResult};

use crate::{TranslateError, translate::TranslateResult};

pub trait SelectionResultExt: Sized {
    fn into_placeholder(self) -> TranslateResult<Placeholder>;
}

impl SelectionResultExt for SelectionResult {
    fn into_placeholder(self) -> TranslateResult<Placeholder> {
        let (_, pv) = self
            .pairs
            .into_iter()
            .next()
            .ok_or_else(|| query_graph_error("SelectionResult is expected to have at least one column"))?;

        match pv {
            PrismaValue::Placeholder(placeholder) => Ok(placeholder),
            _ => Err(query_graph_error(format!(
                "SelectionResult value must be a placeholder, got {pv}"
            ))),
        }
    }
}

pub struct SelectionResults(Vec<SelectionResult>);

impl SelectionResults {
    pub fn new(results: Vec<SelectionResult>) -> Self {
        SelectionResults(results)
    }

    pub fn into_placeholder(self) -> TranslateResult<Placeholder> {
        self.0
            .into_iter()
            .exactly_one()
            .map_err(|e| query_graph_error(format!("expected only one SelectionResult, got {}", e.count())))?
            .into_placeholder()
    }
}

fn query_graph_error(message: impl Into<String>) -> TranslateError {
    TranslateError::GraphBuildError(QueryGraphBuilderError::QueryGraphError(
        QueryGraphError::InvariantViolation(message.into()),
    ))
}
