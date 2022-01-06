#![deny(warnings, rust_2018_idioms)]

pub mod common;
pub mod introspection_engine;
pub mod migration_engine;
#[cfg(feature = "sql")]
pub mod quaint;
pub mod query_engine;

use std::borrow::Cow;

pub use panic_hook::set_panic_hook;

mod panic_hook;

use serde::{Deserialize, Serialize};

pub trait UserFacingError: serde::Serialize {
    const ERROR_CODE: &'static str;

    fn message(&self) -> String;
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct KnownError {
    pub message: String,
    pub meta: serde_json::Value,
    pub error_code: Cow<'static, str>,
}

impl KnownError {
    pub fn new<T: UserFacingError>(inner: T) -> KnownError {
        KnownError {
            message: inner.message(),
            meta: serde_json::to_value(&inner).expect("Failed to render user facing error metadata to JSON"),
            error_code: Cow::from(T::ERROR_CODE),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct UnknownError {
    pub message: String,
    pub backtrace: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Error {
    is_panic: bool,
    #[serde(flatten)]
    inner: ErrorType,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(untagged)]
enum ErrorType {
    Known(KnownError),
    Unknown(UnknownError),
}

impl Error {
    /// Try to interpret the error as a known error.
    pub fn as_known(&self) -> Option<&KnownError> {
        match &self.inner {
            ErrorType::Known(err) => Some(err),
            ErrorType::Unknown(_) => None,
        }
    }

    pub fn message(&self) -> &str {
        match &self.inner {
            ErrorType::Known(err) => &err.message,
            ErrorType::Unknown(err) => &err.message,
        }
    }

    pub fn new_non_panic_with_current_backtrace(message: String) -> Self {
        Error {
            inner: ErrorType::Unknown(UnknownError {
                message,
                backtrace: Some(format!("{:?}", backtrace::Backtrace::new())),
            }),
            is_panic: false,
        }
    }

    pub fn from_dyn_error(err: &dyn std::error::Error) -> Self {
        Error {
            inner: ErrorType::Unknown(UnknownError {
                message: err.to_string(),
                backtrace: None,
            }),
            is_panic: false,
        }
    }

    /// Construct a new UnknownError from a `PanicInfo` in a panic hook. `UnknownError`s created
    /// with this constructor will have a proper, useful backtrace.
    pub fn new_in_panic_hook(panic_info: &std::panic::PanicInfo<'_>) -> Self {
        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| -> String { (*s).to_owned() })
            .or_else(|| panic_info.payload().downcast_ref::<String>().map(|s| s.to_owned()))
            .unwrap_or_else(|| "<unknown panic>".to_owned());

        let backtrace = Some(format!("{:?}", backtrace::Backtrace::new()));
        let location = panic_info
            .location()
            .map(|loc| format!("{}", loc))
            .unwrap_or_else(|| "<unknown location>".to_owned());

        Error {
            inner: ErrorType::Unknown(UnknownError {
                message: format!("[{}] {}", location, message),
                backtrace,
            }),
            is_panic: true,
        }
    }

    /// Build from a KnownError
    pub fn new_known(err: KnownError) -> Self {
        Error {
            inner: ErrorType::Known(err),
            is_panic: false,
        }
    }

    pub fn from_panic_payload(panic_payload: Box<dyn std::any::Any + Send + 'static>) -> Self {
        let message = Self::extract_panic_message(panic_payload).unwrap_or_else(|| "<unknown panic>".to_owned());

        Error {
            inner: ErrorType::Unknown(UnknownError {
                message,
                backtrace: None,
            }),
            is_panic: true,
        }
    }

    pub fn extract_panic_message(panic_payload: Box<dyn std::any::Any + Send + 'static>) -> Option<String> {
        panic_payload
            .downcast_ref::<&str>()
            .map(|s| -> String { (*s).to_owned() })
            .or_else(|| panic_payload.downcast_ref::<String>().map(|s| s.to_owned()))
    }

    /// Extract the inner known error, or panic.
    pub fn unwrap_known(self) -> KnownError {
        match self.inner {
            ErrorType::Known(err) => err,
            err @ ErrorType::Unknown(_) => panic!("Expected known error, got {:?}", err),
        }
    }
}

pub fn new_backtrace() -> backtrace::Backtrace {
    backtrace::Backtrace::new()
}

impl From<UnknownError> for Error {
    fn from(unknown_error: UnknownError) -> Self {
        Error {
            inner: ErrorType::Unknown(unknown_error),
            is_panic: false,
        }
    }
}

impl From<KnownError> for Error {
    fn from(known_error: KnownError) -> Self {
        Error {
            is_panic: false,
            inner: ErrorType::Known(known_error),
        }
    }
}
