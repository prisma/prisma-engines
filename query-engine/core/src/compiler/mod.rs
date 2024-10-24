use query_structure::SelectionResult;

use crate::QueryGraph;

#[derive(Debug)]
pub enum Expression {
    Sequence(Vec<Expression>),
    Query { sql: String, params: SelectionResult },
}

pub fn translate(mut graph: QueryGraph) -> Expression {
    unimplemented!()
}
