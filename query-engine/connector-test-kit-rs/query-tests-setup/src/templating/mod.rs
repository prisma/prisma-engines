mod error;
mod parse_models;
mod parser;

pub use error::*;
pub use parse_models::*;
pub use parser::*;

pub type TemplatingResult<T> = Result<T, TemplatingError>;
