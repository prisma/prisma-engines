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
pub(crate) mod proxy;
pub(crate) mod queryable;
pub(crate) mod send_future;
pub(crate) mod transaction;
pub(crate) mod types;

pub use queryable::from_js;
pub(crate) use transaction::JsTransaction;

#[cfg(target_arch = "wasm32")]
pub use wasm::JsObjectExtern as JsObject;

#[cfg(not(target_arch = "wasm32"))]
pub use ::napi::JsObject;

#[cfg(not(target_arch = "wasm32"))]
pub mod napi;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use napi::*;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::*;

#[cfg(target_arch = "wasm32")]
mod arch {
    pub(crate) use js_sys::JsString;
    use std::str::FromStr;
    use tsify::Tsify;

    pub(crate) fn get_named_property<T>(object: &super::wasm::JsObjectExtern, name: &str) -> JsResult<T>
    where
        T: From<wasm_bindgen::JsValue>,
    {
        Ok(object.get(name.into())?.into())
    }

    pub(crate) fn has_named_property(object: &super::wasm::JsObjectExtern, name: &str) -> JsResult<bool> {
        js_sys::Reflect::has(object, &JsString::from_str(name).unwrap().into())
    }

    pub(crate) fn to_rust_str(value: JsString) -> JsResult<String> {
        Ok(value.into())
    }

    pub(crate) fn from_js_value<C>(value: wasm_bindgen::JsValue) -> C
    where
        C: Tsify + serde::de::DeserializeOwned,
    {
        C::from_js(value).unwrap()
    }

    pub(crate) type JsResult<T> = core::result::Result<T, wasm_bindgen::JsValue>;
}

#[cfg(not(target_arch = "wasm32"))]
mod arch {
    pub(crate) use ::napi::JsString;

    pub(crate) fn get_named_property<T>(object: &::napi::JsObject, name: &str) -> JsResult<T>
    where
        T: ::napi::bindgen_prelude::FromNapiValue,
    {
        object.get_named_property(name)
    }

    pub(crate) fn has_named_property(object: &::napi::JsObject, name: &str) -> JsResult<bool> {
        object.has_named_property(name)
    }

    pub(crate) fn to_rust_str(value: JsString) -> JsResult<String> {
        Ok(value.into_utf8()?.as_str()?.to_string())
    }

    pub(crate) fn from_js_value<C>(value: C) -> C {
        value
    }

    pub(crate) type JsResult<T> = ::napi::Result<T>;
}

pub(crate) use arch::*;
