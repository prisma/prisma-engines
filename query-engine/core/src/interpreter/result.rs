use super::{InterpretationResult, InterpreterError};
use crate::QueryResult;
use prisma_models::prelude::*;

#[derive(Debug, Clone)]
pub enum ExpressionResult {
    Query(QueryResult),
    Computation(ComputationResult),
    Empty,
}

#[derive(Debug, Clone)]
pub enum ComputationResult {
    Diff(DiffResult),
}

/// Diff of two identifier vectors A and B:
/// `left` contains all elements that are in A but not in B.
/// `right` contains all elements that are in B but not in A.
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub left: Vec<RecordProjection>,
    pub right: Vec<RecordProjection>,
}

impl ExpressionResult {
    /// Attempts to transform the result into a vector of record projections.
    pub fn as_projections(&self, model_projection: &ModelProjection) -> InterpretationResult<Vec<RecordProjection>> {
        let converted = match self {
            Self::Query(ref result) => match result {
                QueryResult::Id(id) => match id {
                    Some(id) if model_projection.matches(id) => Some(vec![id.clone()]),
                    None => Some(vec![]),
                    Some(id) => {
                        trace!("RID {:?} does not match MID {:?}", id, model_projection);
                        None
                    }
                },

                // We always select IDs, the unwraps are safe.
                QueryResult::RecordSelection(rs) => Some(
                    rs.scalars
                        .projections(model_projection)
                        .expect("Expected record selection to contain required model ID fields.")
                        .into_iter()
                        .map(|val| val.into())
                        .collect(),
                ),

                _ => None,
            },

            _ => None,
        };

        converted.ok_or(InterpreterError::InterpretationError(
            "Unable to convert result into a set of projections".to_owned(),
            None,
        ))
    }

    pub fn as_query_result(&self) -> InterpretationResult<&QueryResult> {
        let converted = match self {
            Self::Query(ref q) => Some(q),
            _ => None,
        };

        converted.ok_or(InterpreterError::InterpretationError(
            "Unable to convert result into a query result".to_owned(),
            None,
        ))
    }

    pub fn as_diff_result(&self) -> InterpretationResult<&DiffResult> {
        let converted = match self {
            Self::Computation(ComputationResult::Diff(ref d)) => Some(d),
            _ => None,
        };

        converted.ok_or(InterpreterError::InterpretationError(
            "Unable to convert result into a computation result".to_owned(),
            None,
        ))
    }
}
