use napi_derive::napi;

#[derive(serde::Serialize, Clone, Copy)]
#[napi(object)]
pub struct Version {
    pub commit: &'static str,
    pub version: &'static str,
}

#[napi]
pub fn version() -> Version {
    Version {
        commit: env!("GIT_HASH"),
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[napi]
pub fn debug_panic(panic_message: Option<String>) -> napi::Result<()> {
    let user_facing = user_facing_errors::Error::from_panic_payload(Box::new(
        panic_message.unwrap_or_else(|| "query-engine-node-api debug panic".to_string()),
    ));
    let message = serde_json::to_string(&user_facing).unwrap();

    Err(napi::Error::from_reason(message))
}
