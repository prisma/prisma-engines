mod error;
mod expression;
mod expressionista;
mod interpreter;
mod query_interpreters;

pub use error::*;
pub use expression::*;
pub use expressionista::*;
pub use interpreter::*;

type InterpretationResult<T> = std::result::Result<T, InterpreterError>;
