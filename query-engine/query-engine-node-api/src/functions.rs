use crate::error::ApiError;
use napi::{bindgen_prelude::*, JsUnknown};
use napi_derive::napi;
use query_core::{schema::QuerySchemaRef, schema_builder};
use request_handlers::dmmf;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

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
        .map_err(|errors| ApiError::conversion(errors, schema.db.source()))?;

    let internal_data_model = prisma_models::convert(Arc::new(schema));
    let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(internal_data_model, true));
    let dmmf = dmmf::render_dmmf(query_schema);

    Ok(serde_json::to_string(&dmmf)?)
}

#[napi]
pub fn get_config(js_env: Env, options: JsUnknown) -> napi::Result<JsUnknown> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GetConfigOptions {
        datamodel: String,
        #[serde(default)]
        ignore_env_var_errors: bool,
        #[serde(default)]
        datasource_overrides: BTreeMap<String, String>,
        #[serde(default)]
        env: HashMap<String, String>,
    }

    let options: GetConfigOptions = js_env.from_js_value(options)?;

    let GetConfigOptions {
        datamodel,
        ignore_env_var_errors,
        datasource_overrides,
        env,
    } = options;

    let overrides: Vec<(_, _)> = datasource_overrides.into_iter().collect();
    let mut config = psl::parse_configuration(&datamodel).map_err(|errors| ApiError::conversion(errors, &datamodel))?;

    if !ignore_env_var_errors {
        config
            .resolve_datasource_urls_from_env(&overrides, |key| env.get(key).map(ToString::to_string))
            .map_err(|errors| ApiError::conversion(errors, &datamodel))?;
    }

    let serialized = psl::get_config::config_to_mcf_json_value(&config);

    js_env.to_js_value(&serialized)
}

#[napi]
pub fn debug_panic(panic_message: Option<String>) -> napi::Result<()> {
    let user_facing = user_facing_errors::Error::from_panic_payload(Box::new(
        panic_message.unwrap_or_else(|| "query-engine-node-api debug panic".to_string()),
    ));
    let message = serde_json::to_string(&user_facing).unwrap();

    Err(napi::Error::from_reason(message))
}
