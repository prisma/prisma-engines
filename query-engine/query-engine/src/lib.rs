#![allow(clippy::derive_partial_eq_without_eq)]

pub mod cli;
pub mod context;
pub mod error;
pub mod features;
pub mod logger;
pub mod opt;
pub mod server;
pub mod tracer;

use error::PrismaError;

#[macro_use]
extern crate tracing;

pub type PrismaResult<T> = Result<T, PrismaError>;
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum LogFormat {
    Text,
    Json,
}

#[cfg(test)]
mod tests;
