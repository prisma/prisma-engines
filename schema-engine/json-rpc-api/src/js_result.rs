use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use tsify_next::Tsify;

// ---- Common type definitions ----

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[serde(tag = "tag", content = "value", rename_all = "camelCase")]
pub enum JsResult<R, E> {
    Ok(R),
    #[serde(rename = "error")]
    Err(E),
}

impl<R, E> From<Result<R, E>> for JsResult<R, E> {
    fn from(result: Result<R, E>) -> Self {
        match result {
            Ok(r) => JsResult::Ok(r),
            Err(e) => JsResult::Err(e),
        }
    }
}

impl<R, E> From<JsResult<R, E>> for Result<R, E> {
    fn from(val: JsResult<R, E>) -> Self {
        match val {
            JsResult::Ok(r) => Ok(r),
            JsResult::Err(e) => Err(e),
        }
    }
}
