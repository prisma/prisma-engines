mod error;
mod expression;
mod expressionista;
mod interpreter_impl;
mod query_interpreters;

pub(crate) use error::*;
pub(crate) use expressionista::*;
pub(crate) use interpreter_impl::*;

type InterpretationResult<T> = std::result::Result<T, InterpreterError>;
