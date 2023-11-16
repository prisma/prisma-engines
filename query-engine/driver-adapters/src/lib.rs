//! Query Engine Driver Adapters
//! This crate is responsible for defining a `quaint::Connector` implementation that uses functions
//! exposed by client connectors via either `napi-rs` (on native targets) or `wasm_bindgen` / `js_sys` (on Wasm targets).
//!
//! A driver adapter is an object defined in javascript that uses a driver
//! (ex. '@planetscale/database') to provide a similar implementation of that of a `quaint::Connector`. i.e. the ability to query and execute SQL
//! plus some transformation of types to adhere to what a `quaint::Value` expresses.
//!

pub(crate) mod conversion;
pub(crate) mod error;
pub(crate) mod types;

#[cfg(not(target_arch = "wasm32"))]
pub mod napi;

#[cfg(not(target_arch = "wasm32"))]
pub use napi::*;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
