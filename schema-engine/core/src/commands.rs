//! The commands exposed by the schema engine core are defined in this module.

mod diff_cli;

pub use ::commands::*;
pub use diff_cli::diff_cli;

pub use ::commands::{
    core_error::{CoreError, CoreResult},
    GenericApi,
};
