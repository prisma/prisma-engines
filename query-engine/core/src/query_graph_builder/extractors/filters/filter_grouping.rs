use crate::{QueryGraphBuilderError, QueryGraphBuilderResult};
use std::str::FromStr;

pub enum FilterGrouping {
    And,
    Or,
    Not,
}

impl FromStr for FilterGrouping {
    type Err = QueryGraphBuilderError;

    fn from_str(s: &str) -> QueryGraphBuilderResult<Self> {
        match s.to_lowercase().as_str() {
            "and" => Ok(Self::And),
            "or" => Ok(Self::Or),
            "not" => Ok(Self::Not),
            _ => Err(QueryGraphBuilderError::InputError(format!(
                "{} is not a valid grouping filter operation",
                s
            ))),
        }
    }
}
