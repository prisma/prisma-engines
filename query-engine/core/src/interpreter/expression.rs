use super::{result::ExpressionResult, Env};
use crate::{Query, WriteQuery};
use prisma_value::PrismaValue;

type BoxedExpression = Box<Expression>;
type BindingName = String;

/// They do not represent a fundamental building block like `Expression`
/// but custom, build-in functionality.
pub enum FnInvocation {
    Get(BindingName),
    Query(Query),
    Diff(BindingName, BindingName),
    // Raise
    // Inject
    // Filter
}

impl FnInvocation {
    pub fn apply(self, env: Env) -> ExpressionResult {
        // todo result type / dynamic typing?
        todo!()
    }

    // functions defined here
}

/// Fundamental building blocks of the interpreted language.
pub enum Expression {
    Sequence(Vec<Expression>),
    Invoke(FnInvocation),

    // Query {
    //     query: Query,
    // },
    Let {
        bindings: Vec<Binding>,
        inner: BoxedExpression,
    },

    // GetFirstNonEmpty {
    //     binding_names: Vec<String>,
    // },
    If {
        func: BoxedExpression,
        then: BoxedExpression,
        else_: BoxedExpression,
    },
    // Return {
    //     result: ExpressionResult,
    // },
}

impl Expression {
    pub fn raw(query: String, parameters: Vec<PrismaValue>) -> Self {
        let query = Query::Write(WriteQuery::Raw { query, parameters });
        Self::Query { query }
    }
}

pub struct Binding {
    pub name: String,
    pub expr: Expression,
}
