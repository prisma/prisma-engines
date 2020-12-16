use super::{Env, ExpressionResult, InterpretationResult};
use crate::Query;

pub enum Expression {
    Sequence {
        seq: Vec<Expression>,
    },

    Func {
        func: Box<dyn FnOnce(Env) -> InterpretationResult<Expression> + Send + Sync + 'static>,
    },

    Query {
        query: Box<Query>,
    },

    Let {
        bindings: Vec<Binding>,
        expressions: Vec<Expression>,
    },

    Get {
        binding_name: String,
    },

    GetFirstNonEmpty {
        binding_names: Vec<String>,
    },

    If {
        func: Box<dyn FnOnce() -> bool + Send + Sync + 'static>,
        then: Vec<Expression>,
        else_: Vec<Expression>,
    },

    Return {
        result: Box<ExpressionResult>,
    },
}

pub struct Binding {
    pub name: String,
    pub expr: Expression,
}

impl Expression {
    /// Construct a new instance from a `Query`.
    pub fn from_query(query: Query) -> Self {
        Self::Query { query: Box::new(query) }
    }

    /// Construct a new instance from an `ExpressionResult`.
    pub fn from_expression_result(res: ExpressionResult) -> Self {
        Self::Return { result: Box::new(res) }
    }
}
