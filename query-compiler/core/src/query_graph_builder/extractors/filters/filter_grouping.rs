use crate::{QueryGraphBuilderError, QueryGraphBuilderResult};
use schema::constants::filters;
use std::str::FromStr;

#[derive(Debug)]
pub enum FilterGrouping {
    And,
    Or,
    Not,
}

impl FromStr for FilterGrouping {
    type Err = QueryGraphBuilderError;

    fn from_str(s: &str) -> QueryGraphBuilderResult<Self> {
        match s.to_lowercase().as_str() {
            filters::AND_LOWERCASE => Ok(Self::And),
            filters::OR_LOWERCASE => Ok(Self::Or),
            filters::NOT_LOWERCASE => Ok(Self::Not),
            _ => Err(QueryGraphBuilderError::InputError(format!(
                "{s} is not a valid grouping filter operation"
            ))),
        }
    }
}
