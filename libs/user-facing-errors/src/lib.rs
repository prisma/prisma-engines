#![deny(warnings, rust_2018_idioms)]

pub mod common;
pub mod introspection;
pub mod migration_engine;
pub mod quaint;

use serde::Serialize;

pub trait UserFacingError: serde::Serialize {
    const ERROR_CODE: &'static str;

    fn message(&self) -> String;
}

#[derive(Serialize, PartialEq, Debug)]
pub struct KnownError {
    pub message: String,
    pub meta: serde_json::Value,
    pub error_code: &'static str,
}

impl KnownError {
    pub fn new<T: UserFacingError>(inner: T) -> Result<KnownError, serde_json::Error> {
        Ok(KnownError {
            message: inner.message(),
            meta: serde_json::to_value(&inner)?,
            error_code: T::ERROR_CODE,
        })
    }
}

#[derive(Serialize, PartialEq, Debug)]
pub struct UnknownError {
    pub message: String,
    pub backtrace: Option<String>,
}

impl UnknownError {
    /// Construct a new UnknownError from a `PanicInfo` in a panic hook. `UnknownError`s created
    /// with this constructor will have a proper, useful backtrace.
    pub fn new_in_panic_hook(panic_info: &std::panic::PanicInfo<'_>) -> Self {
        let message = Self::extract_panic_message(panic_info.payload()).unwrap_or_else(|| "<unknown panic>".to_owned());
        let backtrace = Some(format!("{:?}", backtrace::Backtrace::new()));
        let location = panic_info
            .location()
            .map(|loc| format!("{}", loc))
            .unwrap_or_else(|| "<unknown location>".to_owned());

        UnknownError {
            message: format!("[{}] {}", location, message),
            backtrace,
        }
    }

    pub fn from_panic_payload(panic_payload: &(dyn std::any::Any + Send + 'static)) -> Self {
        let message = Self::extract_panic_message(panic_payload).unwrap_or_else(|| "<unknown panic>".to_owned());

        UnknownError {
            message,
            backtrace: None,
        }
    }

    pub fn extract_panic_message(panic_payload: &(dyn std::any::Any + Send + 'static)) -> Option<String> {
        panic_payload
            .downcast_ref::<&str>()
            .map(|s| -> String { (*s).to_owned() })
            .or_else(|| panic_payload.downcast_ref::<String>().map(|s| s.to_owned()))
    }
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum Error {
    Known(KnownError),
    Unknown(UnknownError),
}

impl From<UnknownError> for Error {
    fn from(unknown_error: UnknownError) -> Self {
        Error::Unknown(unknown_error)
    }
}

impl From<KnownError> for Error {
    fn from(known_error: KnownError) -> Self {
        Error::Known(known_error)
    }
}
