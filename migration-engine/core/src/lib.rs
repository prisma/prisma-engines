pub mod api;
pub mod cli;
pub mod commands;
mod error;
pub mod migration;
pub mod migration_engine;

use commands::*;
use datamodel::{self, Datamodel};

pub use error::Error;
pub use migration_engine::*;

pub fn parse_datamodel(datamodel: &str) -> CommandResult<Datamodel> {
    let result = datamodel::parse_datamodel_or_pretty_error(&datamodel, "datamodel file, line");
    result.map_err(|e| CommandError::Generic { code: 1001, error: e })
}

pub type Result<T> = std::result::Result<T, Error>;
