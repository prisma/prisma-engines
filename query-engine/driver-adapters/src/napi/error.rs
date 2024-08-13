use futures::{Future, FutureExt};
use napi::Error as NapiError;
use quaint::error::Error as QuaintError;
use std::{any::Any, panic::AssertUnwindSafe};

/// transforms a napi error into a quaint error copying the status and reason
/// properties over
pub(crate) fn into_quaint_error(napi_err: NapiError) -> QuaintError {
    let status = napi_err.status.as_ref().to_owned();
    let reason = napi_err.reason.clone();

    QuaintError::raw_connector_error(status, reason)
}

/// catches a panic thrown during the execution of an asynchronous closure and transforms it into
/// the Error variant of a napi::Result.
pub(crate) async fn async_unwinding_panic<F, R>(fut: F) -> napi::Result<R>
where
    F: Future<Output = napi::Result<R>>,
{
    AssertUnwindSafe(fut)
        .catch_unwind()
        .await
        .unwrap_or_else(panic_to_napi_err)
}

fn panic_to_napi_err<R>(panic_payload: Box<dyn Any + Send>) -> napi::Result<R> {
    panic_payload
        .downcast_ref::<&str>()
        .map(|s| -> String { (*s).to_owned() })
        .or_else(|| panic_payload.downcast_ref::<String>().map(|s| s.to_owned()))
        .map(|message| Err(napi::Error::from_reason(format!("PANIC: {message}"))))
        .ok_or(napi::Error::from_reason("PANIC: unknown panic".to_string()))
        .unwrap()
}
