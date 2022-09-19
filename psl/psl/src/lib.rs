#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

pub use dml::{self, lift, render_datamodel_and_config_to_string, render_datamodel_to_string};
pub use psl_core::*;
