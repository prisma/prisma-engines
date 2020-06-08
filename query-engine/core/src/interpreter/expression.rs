use super::{Env, ExpressionResult, InterpretationResult};
use crate::{Query, RawQueryType, WriteQuery};
use prisma_value::PrismaValue;

pub enum Expression {
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

    Return {
        result: ExpressionResult,
    },
}

impl Expression {
    pub fn raw(query: String, parameters: Vec<PrismaValue>, raw_type: RawQueryType) -> Self {
        let query = Query::Write(WriteQuery::Raw {
            query,
            parameters,
            raw_type,
        });

        Self::Query { query }
    }
}

pub struct Binding {
    pub name: String,
    pub expr: Expression,
}
