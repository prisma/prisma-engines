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

impl std::fmt::Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequence { seq } => f.debug_struct("Sequence").field("seq", seq).finish(),
            Self::Func { func } => f.debug_struct("Func").finish(),
            Self::Query { query } => f.debug_struct("Query").field("query", query).finish(),
            Self::Let { bindings, expressions } => f
                .debug_struct("Let")
                .field("bindings", bindings)
                .field("expressions", expressions)
                .finish(),
            Self::Get { binding_name } => f.debug_struct("Get").field("binding_name", binding_name).finish(),
            Self::GetFirstNonEmpty { binding_names } => f
                .debug_struct("GetFirstNonEmpty")
                .field("binding_names", binding_names)
                .finish(),
            Self::If { func, then, else_ } => f
                .debug_struct("If")
                // .field("func", func)
                .field("then", then)
                .field("else_", else_)
                .finish(),
            Self::Return { result } => f.debug_struct("Return").field("result", result).finish(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Binding {
    pub name: String,
    pub expr: Expression,
}
