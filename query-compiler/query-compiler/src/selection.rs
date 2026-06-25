use query_core::{QueryGraphBuilderError, QueryGraphError};
use query_structure::Placeholder;

use crate::{TranslateError, translate::TranslateResult};

pub fn projected_placeholder(placeholder: Option<Placeholder>, label: &str) -> TranslateResult<Placeholder> {
    placeholder.ok_or_else(|| query_graph_error(format!("{label} node is missing projected placeholder input")))
}

fn query_graph_error(message: impl Into<String>) -> TranslateError {
    TranslateError::GraphBuildError(QueryGraphBuilderError::QueryGraphError(
        QueryGraphError::InvariantViolation(message.into()),
    ))
}
