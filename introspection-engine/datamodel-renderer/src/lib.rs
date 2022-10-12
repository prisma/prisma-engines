//! A set of datastructures meant for rendering a Prisma data model as
//! a string. We don't even try to make the result pretty. Please use
//! the functionality of prisma-fmt for that.
//!
//! All structs implement `std::fmt::Display` for easy usage.

#![warn(missing_docs)]

mod configuration;
mod datasource;
mod generator;
mod value;

pub use configuration::Configuration;
pub use datasource::Datasource;
pub use generator::Generator;
pub use value::{Array, Commented, Env, Function, FunctionParam, RelationMode, Text, Value};
