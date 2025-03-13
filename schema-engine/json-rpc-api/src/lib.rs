//! JSON-RPC API definitions for the Prisma Schema Engine.
//!
//! This crate defines the JSON-RPC API for the Prisma Schema Engine, including
//! all method definitions, request parameters, and response types.

mod js_result;
pub mod migration_directory;

/// API type definitions used by the methods.
pub mod types;

/// JSON-RPC API method definitions.
pub mod method_names;
