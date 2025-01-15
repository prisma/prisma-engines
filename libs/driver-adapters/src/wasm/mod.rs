//! Query Engine Driver Adapters: `wasm`-specific implementation.

mod adapter_method;
mod conversion;
mod error;
mod from_js;
mod js_object_extern;
pub(crate) mod result;
mod to_js;

pub(crate) use adapter_method::AdapterMethod;
pub(crate) use from_js::FromJsValue;
pub use js_object_extern::JsObjectExtern;
