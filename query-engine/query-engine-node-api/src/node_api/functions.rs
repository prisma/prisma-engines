use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use datamodel_connector::ConnectorCapabilities;
use napi::{CallContext, JsString, JsUnknown};
use napi_derive::js_function;
use prisma_models::DatamodelConverter;
use query_core::{schema_builder, BuildMode, QuerySchemaRef};
use request_handlers::dmmf;

use crate::error::ApiError;

#[js_function(0)]
pub fn version(ctx: CallContext) -> napi::Result<JsUnknown> {
    #[derive(serde::Serialize, Clone, Copy)]
    struct Version {
        commit: &'static str,
        version: &'static str,
    }

    let version = Version {
        commit: env!("GIT_HASH"),
        version: env!("CARGO_PKG_VERSION"),
    };

    ctx.env.to_js_value(&version)
}

#[js_function(1)]
pub fn dmmf(ctx: CallContext) -> napi::Result<JsString> {
    let datamodel_string = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;

    let datamodel = datamodel::parse_datamodel(&datamodel_string)
        .map_err(|errors| ApiError::conversion(errors, &datamodel_string))?;

    let template = DatamodelConverter::convert(&datamodel.subject);

    let config = datamodel::parse_configuration(&datamodel_string)
        .map_err(|errors| ApiError::conversion(errors, &datamodel_string))?;

    let capabilities = match config.subject.datasources.first() {
        Some(datasource) => datasource.capabilities(),
        None => ConnectorCapabilities::empty(),
    };

    let internal_data_model = template.build("".into());

    let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
        internal_data_model,
        BuildMode::Modern,
        true,
        capabilities,
        config.subject.preview_features().cloned().collect(),
    ));

    let dmmf = dmmf::render_dmmf(&datamodel.subject, query_schema);
    let dmmf_string = serde_json::to_string(&dmmf).unwrap();

    ctx.env.adjust_external_memory(dmmf_string.len() as i64)?;
    ctx.env.create_string_from_std(dmmf_string)
}

#[js_function(1)]
pub fn get_config(ctx: CallContext) -> napi::Result<JsUnknown> {
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

    let options = ctx.get::<JsUnknown>(0)?;
    let options: GetConfigOptions = ctx.env.from_js_value(options)?;

    let GetConfigOptions {
        datamodel,
        ignore_env_var_errors,
        datasource_overrides,
        env,
    } = options;

    let overrides: Vec<(_, _)> = datasource_overrides.into_iter().collect();
    let mut config =
        datamodel::parse_configuration(&datamodel).map_err(|errors| ApiError::conversion(errors, &datamodel))?;

    if !ignore_env_var_errors {
        config
            .subject
            .resolve_datasource_urls_from_env(&overrides, |key| env.get(key).map(ToString::to_string))
            .map_err(|errors| ApiError::conversion(errors, &datamodel))?;
    }

    let serialized = datamodel::json::mcf::config_to_mcf_json_value(&config);
    let s = serde_json::to_string(&serialized).unwrap();

    ctx.env.adjust_external_memory(s.len() as i64)?;
    ctx.env.to_js_value(&serialized)
}
