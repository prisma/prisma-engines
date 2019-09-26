use super::*;
use std::convert::TryFrom;

impl TryFrom<Node> for Query {
    type Error = QueryGraphError;

    fn try_from(n: Node) -> QueryGraphResult<Query> {
        match n {
            Node::Query(q) => Ok(q),
            x => Err(QueryGraphError::InvalidNodeTransformation {
                from: format!("{}", x),
                to: "Query".to_owned(),
            }),
        }
    }
}

impl TryFrom<Node> for Flow {
    type Error = QueryGraphError;

    fn try_from(n: Node) -> QueryGraphResult<Flow> {
        match n {
            Node::Flow(f) => Ok(f),
            x => Err(QueryGraphError::InvalidNodeTransformation {
                from: format!("{}", x),
                to: "Flow".to_owned(),
            }),
        }
    }
}
