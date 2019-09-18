mod expressionista;
mod expression;
mod formatters;
mod interpreter;
mod error;

pub(self) mod query_interpreters;

pub use expressionista::*;
pub use expression::*;
pub use formatters::*;
pub use interpreter::*;
pub use error::*;

type InterpretationResult<T> = std::result::Result<T, InterpreterError>;