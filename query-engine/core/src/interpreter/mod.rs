mod error;
mod expression;
mod expressionista;
mod interpreter;
mod query_interpreters;

pub(crate) use error::*;
pub(crate) use expressionista::*;
pub(crate) use interpreter::*;

type InterpretationResult<T> = std::result::Result<T, InterpreterError>;
