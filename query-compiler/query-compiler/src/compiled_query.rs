use serde::Serialize;
use crate::Expression;
use crate::result_node::ResultNode;

#[derive(Debug, Serialize)]
pub struct CompiledQuery {
    expression: Expression,
    result_structure: Option<ResultNode>,
}

impl CompiledQuery {
    pub fn new(expression: Expression, result_structure: Option<ResultNode>) -> Self {
        Self {
            expression,
            result_structure,
        }
    }
}
