use super::Env;
use crate::Query;

pub enum Expression {
    Sequence {
        seq: Vec<Expression>,
    },

    Func {
        func: Box<dyn FnOnce(Env) -> Expression>,
    },

    Query {
        query: Query,
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
        func: Box<dyn FnOnce() -> bool>,
        then: Vec<Expression>,
        else_: Vec<Expression>,
    },
}

pub struct Binding {
    pub name: String,
    pub expr: Expression,
}
