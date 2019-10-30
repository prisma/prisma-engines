mod error;
mod expression;
mod expressionista;
mod formatters;
mod interpreter;

pub(self) mod query_interpreters;

pub use error::*;
pub use expression::*;
pub use expressionista::*;
pub use formatters::*;
pub use interpreter::*;

type InterpretationResult<T> = std::result::Result<T, InterpreterError>;
