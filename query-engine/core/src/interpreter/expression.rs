use super::{result::ExpressionResult, Env};
use crate::{Query, QueryResult, WriteQuery};
use prisma_models::ModelProjection;
use prisma_value::PrismaValue;

type BoxedExpression = Box<Expression>;
type BindingName = String;

/// They do not represent a fundamental building block like `Expression`,
/// but build-in functionality to form the core logic of the program.
#[derive(Debug)]
pub enum FnInvocation {
    Get(BindingName),
    Query(BoxedExpression),
    Diff(BindingName, BindingName),
    TransformQuery(Query, Vec<QueryTransformer>),
    Raise(String),
}

#[derive(Debug)]
pub enum QueryTransformer {
    InjectData(ModelProjection, BindingName),
    InjectFilter(ModelProjection, BindingName),
}

impl FnInvocation {
    pub fn apply(self, env: Env) -> Expression {
        // todo result type / dynamic typing?
        todo!()
    }

    // functions defined here
}

/// Fundamental building blocks of the interpreter.
#[derive(Debug)]
pub enum Expression {
    Sequence(Vec<Expression>),
    Invoke(FnInvocation),

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
    /// Simply evaluates to the contained data.
    Data(Data),
}

#[derive(Debug)]
pub enum Data {
    Query(Query),
    Bool(bool),
    ResultSet(QueryResult),
}

#[derive(Debug)]
pub struct Binding {
    pub name: String,
    pub expr: Expression,
}
