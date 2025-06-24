use super::{Env, ExpressionResult, InterpretationResult};
use crate::Query;

pub(crate) enum Expression {
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

impl Expression {
    pub fn is_empty_seq(&self) -> bool {
        matches!(self, Expression::Sequence { seq } if seq.is_empty())
    }
}

pub(crate) struct Binding {
    pub name: String,
    pub expr: Expression,
}
