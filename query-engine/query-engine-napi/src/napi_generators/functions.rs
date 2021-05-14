use std::{collections::BTreeMap, sync::Arc};

use datamodel_connector::ConnectorCapabilities;
use napi::{CallContext, JsString, JsUnknown};
use napi_derive::js_function;
use prisma_models::DatamodelConverter;
use query_core::{exec_loader, schema_builder, BuildMode, QuerySchemaRef};
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
pub fn dmmf(ctx: CallContext) -> napi::Result<JsUnknown> {
    let datamodel = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;
    let template = DatamodelConverter::convert_string(datamodel.clone());

    let config =
        datamodel::parse_configuration(&datamodel).map_err(|errors| ApiError::conversion(errors, &datamodel))?;

    let capabilities = match config.subject.datasources.first() {
        Some(datasource) => datasource.capabilities(),
        None => ConnectorCapabilities::empty(),
    };

    let source = config
        .subject
        .datasources
        .first()
        .ok_or_else(|| ApiError::configuration("No valid data source found"))?;

    let url = source
        .load_url()
        .map_err(|err| ApiError::Conversion(err, datamodel.clone()))?;

    let db_name = exec_loader::db_name(source, &url).map_err(ApiError::from)?;
    let internal_data_model = template.build(db_name);

    let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
        internal_data_model,
        BuildMode::Modern,
        true,
        capabilities,
        config.subject.preview_features().cloned().collect(),
    ));

    let dm = datamodel::parse_datamodel(datamodel.as_str()).unwrap();
    let dmmf = dmmf::render_dmmf(&dm.subject, query_schema);

    ctx.env.to_js_value(&dmmf)
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
    }

    let options = ctx.get::<JsUnknown>(0)?;
    let options: GetConfigOptions = ctx.env.from_js_value(options)?;

    let GetConfigOptions {
        datamodel,
        ignore_env_var_errors,
        datasource_overrides,
    } = options;

    let overrides: Vec<(_, _)> = datasource_overrides.into_iter().collect();
    let mut config = datamodel::parse_configuration_with_url_overrides(&datamodel, overrides)
        .map_err(|errors| ApiError::conversion(errors, &datamodel))?;

    config.subject = config
        .subject
        .validate_that_one_datasource_is_provided()
        .map_err(|errors| ApiError::conversion(errors, &datamodel))?;

    if !ignore_env_var_errors {
        config
            .subject
            .resolve_datasource_urls_from_env()
            .map_err(|errors| ApiError::conversion(errors, &datamodel))?;
    }

    let serialized = datamodel::json::mcf::config_to_mcf_json_value(&config);
    ctx.env.to_js_value(&serialized)
}
