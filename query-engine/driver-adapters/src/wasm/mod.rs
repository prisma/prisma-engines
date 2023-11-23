//! Query Engine Driver Adapters: `wasm`-specific implementation.

mod async_js_function;
mod error;
mod from_js;
mod js_object_extern;
pub(crate) mod proxy;
mod result;
mod transaction;

pub use crate::queryable::{from_wasm, JsQueryable};
pub use js_object_extern::JsObjectExtern;
