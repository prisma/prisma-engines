//! Query Engine Driver Adapters: `napi`-specific implementation.

mod adapter_method;
mod conversion;
mod error;
pub(crate) mod result;

pub(crate) use adapter_method::AdapterMethod;
