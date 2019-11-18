use super::{Env, InterpretationResult};
use crate::Query;

pub enum Expression {
    // A general function concept will replace most other parts in the future.
    FnDef {
        parameters: Vec<Parameter>,
        body: Box<Expression>,
    },

    Fn {
        arguments: Vec<Value>,
    },

    Sequence {
        seq: Vec<Expression>,
    },

    Func {
        func: Box<dyn FnOnce(Env) -> InterpretationResult<Expression> + Send + Sync + 'static>,
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
        func: Box<dyn FnOnce() -> bool + Send + Sync + 'static>,
        then: Vec<Expression>,
        else_: Vec<Expression>,
    },
}

pub struct Binding {
    pub name: String,
    pub expr: Expression,
}

// Everything below is experimental.

pub struct Parameter {
    name: String,
    typ: Typ,
}

trait TypeCoercion<T> {
    fn coerce(self, param: &Parameter) -> T;
}

pub enum Value {}

// Reuse schema types?
pub enum Typ {
    String,
    Int,
    List(Vec<Typ>),
}
