use super::{result::ExpressionResult, Env};
use crate::{Query, WriteQuery};
use prisma_value::PrismaValue;

type BoxedExpression = Box<Expression>;
type BindingName = String;

/// They do not represent a fundamental building block like `Expression`
/// but build-in functionality to form the core logic of the program.
pub enum FnInvocation {
    Get(BindingName),
    Query(Query),
    Diff(BindingName, BindingName),
    // Raise for runtime interpretation errors
    // Inject for argument injection
    // Filter for injecting filters
}

impl FnInvocation {
    pub fn apply(self, env: Env) -> ExpressionResult {
        // todo result type / dynamic typing?
        todo!()
    }

    // functions defined here
}

/// Fundamental building blocks of the interpreter.
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

pub struct Binding {
    pub name: String,
    pub expr: Expression,
}
