use crate::query_document::{ParsedArgument, ParsedInputValue};
use crate::query_graph_builder::{QueryGraphBuilderError, QueryGraphBuilderResult};
use query_structure::PrismaValue;

pub(crate) fn validate_limit(limit_arg: Option<ParsedArgument<'_>>) -> QueryGraphBuilderResult<Option<usize>> {
    let limit = limit_arg.and_then(|limit_arg| match limit_arg.value {
        ParsedInputValue::Single(PrismaValue::Int(i)) => Some(i),
        _ => None,
    });

    match limit {
        Some(i) => {
            if i < 0 {
                return Err(QueryGraphBuilderError::InputError(format!(
                    "Provided limit ({i}) must be a positive integer."
                )));
            }

            match usize::try_from(i) {
                Ok(i) => Ok(Some(i)),
                Err(_) => Err(QueryGraphBuilderError::InputError(format!(
                    "Provided limit ({}) is beyond max int value for platform ({}).",
                    i,
                    usize::MAX
                ))),
            }
        }
        None => Ok(None),
    }
}
