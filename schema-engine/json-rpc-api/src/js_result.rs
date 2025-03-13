use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use tsify_next::Tsify;

// ---- Common type definitions ----

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[serde(tag = "tag", content = "value", rename_all = "camelCase")]
pub enum JSResult<R, E> {
    Ok(R),
    Err(E),
}

impl<R, E> From<Result<R, E>> for JSResult<R, E> {
    fn from(result: Result<R, E>) -> Self {
        match result {
            Ok(r) => JSResult::Ok(r),
            Err(e) => JSResult::Err(e),
        }
    }
}

impl<R, E> From<JSResult<R, E>> for Result<R, E> {
    fn from(val: JSResult<R, E>) -> Self {
        match val {
            JSResult::Ok(r) => Ok(r),
            JSResult::Err(e) => Err(e),
        }
    }
}
