use crate::error::ApiError;
use napi_derive::napi;
use request_handlers::dmmf;
use std::sync::Arc;

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
pub fn dmmf(datamodel_string: String) -> napi::Result<String> {
    let mut schema = psl::validate(datamodel_string.into());

    schema
        .diagnostics
        .to_result()
        .map_err(|errors| ApiError::conversion(errors, schema.db.source_assert_single()))?;

    let query_schema = query_core::schema::build(Arc::new(schema), true);
    let dmmf = dmmf::render_dmmf(&query_schema);

    Ok(serde_json::to_string(&dmmf)?)
}

#[napi]
pub fn debug_panic(panic_message: Option<String>) -> napi::Result<()> {
    let user_facing = user_facing_errors::Error::from_panic_payload(Box::new(
        panic_message.unwrap_or_else(|| "query-engine-node-api debug panic".to_string()),
    ));
    let message = serde_json::to_string(&user_facing).unwrap();

    Err(napi::Error::from_reason(message))
}
