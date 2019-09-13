use super::*;
use std::convert::TryFrom;

pub type TransformationResult<T> = std::result::Result<T, QueryGraphError>;

impl TryFrom<Node> for Query {
    type Error = QueryGraphError;

    fn try_from(n: Node) -> TransformationResult<Query> {
        match n {
            Node::Query(q) => Ok(q),
            x => Err(QueryGraphError::InvalidTransformation { from: format!("{}", x), to: "Query".to_owned() }),
        }
    }
}

impl TryFrom<Node> for Flow {
    type Error = QueryGraphError;

    fn try_from(n: Node) -> TransformationResult<Flow> {
        match n {
            Node::Flow(f) => Ok(f),
            x => Err(QueryGraphError::InvalidTransformation { from: format!("{}", x), to: "Flow".to_owned() }),
        }
    }
}