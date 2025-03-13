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
pub(crate) mod factory;
pub(crate) mod proxy;
pub(crate) mod queryable;
pub(crate) mod send_future;
pub(crate) mod transaction;
pub(crate) mod types;

use crate::error::DriverAdapterError;
use quaint::error::{Error as QuaintError, ErrorKind};

#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::result::AdapterResult;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use napi::result::AdapterResult;

impl From<DriverAdapterError> for QuaintError {
    fn from(value: DriverAdapterError) -> Self {
        match value {
            DriverAdapterError::UnsupportedNativeDataType { native_type } => {
                QuaintError::builder(ErrorKind::UnsupportedColumnType {
                    column_type: native_type,
                })
                .build()
            }
            DriverAdapterError::InvalidIsolationLevel { level } => {
                QuaintError::builder(ErrorKind::InvalidIsolationLevel(level)).build()
            }
            DriverAdapterError::GenericJs { id } => QuaintError::external_error(id),
            #[cfg(feature = "postgresql")]
            DriverAdapterError::Postgres(e) => e.into(),
            #[cfg(feature = "mysql")]
            DriverAdapterError::Mysql(e) => e.into(),
            #[cfg(feature = "sqlite")]
            DriverAdapterError::Sqlite(e) => e.into(),
            // in future, more error types would be added and we'll need to convert them to proper QuaintErrors here
        }
    }
}

pub use factory::JsAdapterFactory;
pub use queryable::{queryable_from_js, JsQueryable};
pub(crate) use transaction::JsTransaction;
pub use types::AdapterProvider;

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
    use tsify_next::Tsify;

    pub(crate) fn get_named_property<T>(object: &super::wasm::JsObjectExtern, name: &str) -> JsResult<T>
    where
        T: From<wasm_bindgen::JsValue>,
    {
        Ok(object.get(name.into())?.into())
    }

    pub(crate) fn get_optional_named_property<T>(
        object: &super::wasm::JsObjectExtern,
        name: &str,
    ) -> JsResult<Option<T>>
    where
        T: From<wasm_bindgen::JsValue>,
    {
        if has_named_property(object, name)? {
            Ok(Some(get_named_property(object, name)?))
        } else {
            Ok(None)
        }
    }

    fn has_named_property(object: &super::wasm::JsObjectExtern, name: &str) -> JsResult<bool> {
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
        T: ::napi::bindgen_prelude::FromNapiValue + ::napi::bindgen_prelude::ValidateNapiValue,
    {
        object.get_named_property(name)
    }

    pub(crate) fn get_optional_named_property<T>(object: &::napi::JsObject, name: &str) -> JsResult<Option<T>>
    where
        T: ::napi::bindgen_prelude::FromNapiValue + ::napi::bindgen_prelude::ValidateNapiValue,
    {
        if has_named_property(object, name)? {
            Ok(Some(get_named_property(object, name)?))
        } else {
            Ok(None)
        }
    }

    fn has_named_property(object: &::napi::JsObject, name: &str) -> JsResult<bool> {
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
