use crate::error::ApiError;
use napi::{bindgen_prelude::*, JsUnknown};
use napi_derive::napi;
use prisma_models::InternalDataModelBuilder;
use psl::datamodel_connector::ConnectorCapabilities;
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
    let datamodel =
        psl::parse_datamodel(&datamodel_string).map_err(|errors| ApiError::conversion(errors, &datamodel_string))?;

    let config = psl::parse_configuration(&datamodel_string)
        .map_err(|errors| ApiError::conversion(errors, &datamodel_string))?;
    let datasource = config.subject.datasources.first();

    let capabilities = datasource
        .map(|ds| ds.capabilities())
        .unwrap_or_else(ConnectorCapabilities::empty);

    let referential_integrity = datasource.map(|ds| ds.referential_integrity()).unwrap_or_default();

    let internal_data_model = InternalDataModelBuilder::from(&datamodel.subject).build("".into());

    let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
        internal_data_model,
        true,
        capabilities,
        config.subject.preview_features().iter().collect(),
        referential_integrity,
    ));

    let dmmf = dmmf::render_dmmf(&datamodel.subject, query_schema);

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
            .subject
            .resolve_datasource_urls_from_env(&overrides, |key| env.get(key).map(ToString::to_string))
            .map_err(|errors| ApiError::conversion(errors, &datamodel))?;
    }

    let serialized = psl::mcf::config_to_mcf_json_value(&config);

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
