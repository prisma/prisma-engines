use crate::error::ApiError;
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub commit: &'static str,
    pub version: &'static str,
}

#[wasm_bindgen]
pub fn version() -> Version {
    Version {
        commit: env!("GIT_HASH"),
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[wasm_bindgen]
pub fn dmmf(datamodel_string: String) -> Result<String, wasm_bindgen::JsError> {
    let mut schema = psl::validate(datamodel_string.into());

    schema
        .diagnostics
        .to_result()
        .map_err(|errors| ApiError::conversion(errors, schema.db.source()))?;

    Ok("{}".to_string())

    // let query_schema = query_core::schema::build(Arc::new(schema), true);
    // let dmmf = dmmf::render_dmmf(&query_schema);

    // Ok(serde_json::to_string(&dmmf)?)
}

#[wasm_bindgen]
pub fn debug_panic(panic_message: Option<String>) -> Result<(), wasm_bindgen::JsError> {
    let user_facing = user_facing_errors::Error::from_panic_payload(Box::new(
        panic_message.unwrap_or_else(|| "query-engine-wasm debug panic".to_string()),
    ));
    let message = serde_json::to_string(&user_facing).unwrap();

    Err(wasm_bindgen::JsError::new(&message))
}
