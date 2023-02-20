use napi::{Env, JsFunction, JsUnknown};
use napi_derive::napi;

fn not_a_real_qe() -> napi::Error {
    napi::Error::from_reason("Not a real Query Engine")
}

#[napi]
pub struct QueryEngine;

#[napi]
impl QueryEngine {
    #[napi(constructor)]
    pub fn new(_napi_env: Env, _options: JsUnknown, _callback: JsFunction) -> napi::Result<Self> {
        Ok(Self)
    }

    #[napi]
    pub async fn connect(&self, _trace: String) -> napi::Result<()> {
        Ok(())
    }

    #[napi]
    pub async fn disconnect(&self, _trace: String) -> napi::Result<()> {
        Ok(())
    }

    #[napi]
    pub async fn query(&self, _trace: String, _tx_id: Option<String>) -> napi::Result<String> {
        Err(not_a_real_qe())
    }

    #[napi]
    pub async fn start_transaction(&self, _input: String, _trace: String) -> napi::Result<String> {
        Err(not_a_real_qe())
    }

    #[napi]
    pub async fn commit_transaction(&self, _tx_id: String, _trace: String) -> napi::Result<String> {
        Err(not_a_real_qe())
    }

    #[napi]
    pub async fn rollback_transaction(&self, _tx_id: String, _trace: String) -> napi::Result<String> {
        Err(not_a_real_qe())
    }

    #[napi]
    pub async fn dmmf(&self) -> napi::Result<String> {
        Err(not_a_real_qe())
    }

    #[napi]
    pub async fn sdl_schema(&self) -> napi::Result<String> {
        Err(not_a_real_qe())
    }

    #[napi]
    pub async fn metrics(&self) -> napi::Result<String> {
        Err(not_a_real_qe())
    }
}

#[derive(serde::Serialize, Clone, Copy)]
#[napi(object)]
pub struct Version {
    pub commit: &'static str,
    pub version: &'static str,
}

#[napi]
pub fn version() -> Version {
    Version {
        commit: "0000000000000000000000000000000000000000",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[napi]
pub fn dmmf(_datamodel_string: String) -> napi::Result<String> {
    Err(not_a_real_qe())
}

#[napi]
pub fn get_config(_js_env: Env, _options: JsUnknown) -> napi::Result<JsUnknown> {
    Err(not_a_real_qe())
}

#[napi]
pub fn debug_panic(panic_message: Option<String>) -> napi::Result<()> {
    let panic_message = panic_message.unwrap_or_default();
    panic!("debug panic: {panic_message}")
}

#[napi]
pub fn literally_do_nothing() {
    nope::nops();
}
