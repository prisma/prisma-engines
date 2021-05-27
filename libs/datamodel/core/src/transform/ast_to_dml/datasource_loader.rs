use super::{
    super::helpers::{ValueListValidator, ValueValidator},
    builtin_datasource_providers::{
        MongoDbDatasourceProvider, MsSqlDatasourceProvider, MySqlDatasourceProvider, PostgresDatasourceProvider,
        SqliteDatasourceProvider,
    },
    datasource_provider::DatasourceProvider,
};
use crate::{
    ast::SourceConfig,
    diagnostics::{DatamodelError, Diagnostics, ValidatedDatasource, ValidatedDatasources},
};
use crate::{ast::Span, common::preview_features::PreviewFeature, configuration::StringFromEnvVar};
use crate::{
    ast::{self},
    Datasource,
};
use std::collections::{HashMap, HashSet};

const PREVIEW_FEATURES_KEY: &str = "previewFeatures";
const SHADOW_DATABASE_URL_KEY: &str = "shadowDatabaseUrl";
const URL_KEY: &str = "url";

/// Is responsible for loading and validating Datasources defined in an AST.
pub struct DatasourceLoader {
    source_definitions: Vec<Box<dyn DatasourceProvider>>,
}

impl DatasourceLoader {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            source_definitions: get_builtin_datasource_providers(),
        }
    }

    /// Loads all datasources from the provided schema AST.
    /// - `ignore_datasource_urls`: datasource URLs are not parsed. They are replaced with dummy values.
    /// - `datasource_url_overrides`: datasource URLs are not parsed and overridden with the provided ones.
    pub fn load_datasources_from_ast(
        &self,
        ast_schema: &ast::SchemaAst,
        preview_features: &HashSet<&PreviewFeature>,
    ) -> Result<ValidatedDatasources, Diagnostics> {
        let mut sources = vec![];
        let mut diagnostics = Diagnostics::new();

        for src in &ast_schema.sources() {
            match self.lift_datasource(&src, preview_features) {
                Ok(loaded_src) => {
                    diagnostics.append_warning_vec(loaded_src.warnings);
                    sources.push(loaded_src.subject)
                }
                // Lift error.
                Err(err) => {
                    for e in err.errors {
                        match e {
                            DatamodelError::ArgumentNotFound { argument_name, span } => {
                                diagnostics.push_error(DatamodelError::new_source_argument_not_found_error(
                                    argument_name.as_str(),
                                    src.name.name.as_str(),
                                    span,
                                ));
                            }
                            _ => {
                                diagnostics.push_error(e);
                            }
                        }
                    }
                    diagnostics.append_warning_vec(err.warnings)
                }
            }
        }

        if sources.len() > 1 {
            for src in &ast_schema.sources() {
                diagnostics.push_error(DatamodelError::new_source_validation_error(
                    &"You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.".to_string(),
                    &src.name.name,
                    src.span,
                ));
            }
        }

        if diagnostics.has_errors() {
            Err(diagnostics)
        } else {
            Ok(ValidatedDatasources {
                subject: sources,
                warnings: diagnostics.warnings,
            })
        }
    }

    fn lift_datasource(
        &self,
        ast_source: &ast::SourceConfig,
        preview_features: &HashSet<&PreviewFeature>,
    ) -> Result<ValidatedDatasource, Diagnostics> {
        let source_name = &ast_source.name.name;
        let args: HashMap<_, _> = ast_source
            .properties
            .iter()
            .map(|arg| (arg.name.name.as_str(), ValueValidator::new(&arg.value)))
            .collect();
        let diagnostics = Diagnostics::new();

        let provider_arg = args
            .get("provider")
            .ok_or_else(|| DatamodelError::new_argument_not_found_error("provider", ast_source.span))?;

        if provider_arg.is_from_env() {
            return Err(diagnostics.merge_error(DatamodelError::new_functional_evaluation_error(
                &"A datasource must not use the env() function in the provider argument.".to_string(),
                ast_source.span,
            )));
        }

        let provider = match provider_arg.as_string_literal() {
            Some(("", _)) => {
                return Err(diagnostics.merge_error(DatamodelError::new_source_validation_error(
                    "The provider argument in a datasource must not be empty",
                    source_name,
                    provider_arg.span(),
                )));
            }
            None => {
                return Err(diagnostics.merge_error(DatamodelError::new_source_validation_error(
                    "The provider argument in a datasource must be a string literal",
                    source_name,
                    provider_arg.span(),
                )));
            }
            Some((provider, _)) => provider,
        };

        let url_arg = args
            .get(URL_KEY)
            .ok_or_else(|| DatamodelError::new_argument_not_found_error(URL_KEY, ast_source.span))?;

        let url = match url_arg.as_str_from_env() {
            Ok(str_from_env_var) => str_from_env_var,
            Err(err) => {
                return Err(diagnostics.merge_error(err));
            }
        };

        let shadow_database_url_arg = args.get(SHADOW_DATABASE_URL_KEY);

        let shadow_database_url: Option<(StringFromEnvVar, Span)> =
            if let Some(shadow_database_url_arg) = shadow_database_url_arg.as_ref() {
                match shadow_database_url_arg.as_str_from_env() {
                    Ok(shadow_database_url) => Some(shadow_database_url)
                        .filter(|s| !s.as_literal().map(|lit| lit.is_empty()).unwrap_or(false))
                        .map(|url| (url, shadow_database_url_arg.span())),

                    // We intentionally ignore the shadow database URL if it is defined in an env var that is missing.
                    Err(DatamodelError::EnvironmentFunctionalEvaluationError { .. }) => None,

                    Err(err) => {
                        return Err(diagnostics.merge_error(err));
                    }
                }
            } else {
                None
            };

        preview_features_guardrail(&args)?;

        let documentation = ast_source.documentation.as_ref().map(|comment| comment.text.clone());

        let datasource_provider = self.get_datasource_provider(&provider).ok_or_else(|| {
            diagnostics
                .clone()
                .merge_error(DatamodelError::new_datasource_provider_not_known_error(
                    provider,
                    provider_arg.span(),
                ))
        })?;

        Ok(ValidatedDatasource {
            subject: Datasource {
                name: source_name.to_string(),
                provider: provider.to_owned(),
                active_provider: datasource_provider.canonical_name().to_owned(),
                url,
                url_span: url_arg.span(),
                documentation,
                active_connector: datasource_provider.connector(),
                shadow_database_url,
                planet_scale_mode: get_planet_scale_mode_arg(&args, preview_features, ast_source)?,
            },
            warnings: diagnostics.warnings,
        })
    }

    fn get_datasource_provider(&self, provider: &str) -> Option<&dyn DatasourceProvider> {
        self.source_definitions
            .iter()
            .find(|sd| sd.is_provider(provider))
            .map(|b| b.as_ref())
    }
}

fn get_builtin_datasource_providers() -> Vec<Box<dyn DatasourceProvider>> {
    vec![
        Box::new(MySqlDatasourceProvider::new()),
        Box::new(PostgresDatasourceProvider::new()),
        Box::new(SqliteDatasourceProvider::new()),
        Box::new(MsSqlDatasourceProvider::new()),
        Box::new(MongoDbDatasourceProvider::new()),
    ]
}

const PLANET_SCALE_PREVIEW_FEATURE_ERR: &str = r#"
The `planetScaleMode` option can only be set if the preview feature is enabled in a generator block.

Example:

generator client {
    provider = "prisma-client-js"
    previewFeatures = ["planetScaleMode"]
}
"#;

fn get_planet_scale_mode_arg(
    args: &HashMap<&str, ValueValidator>,
    preview_features: &HashSet<&PreviewFeature>,
    source: &SourceConfig,
) -> Result<bool, DatamodelError> {
    let arg = args.get("planetScaleMode");

    match arg {
        None => Ok(false),
        Some(value) => {
            let mode_enabled = value.as_bool()?;

            if mode_enabled && !preview_features.contains(&PreviewFeature::PlanetScaleMode) {
                return Err(DatamodelError::new_source_validation_error(
                    PLANET_SCALE_PREVIEW_FEATURE_ERR,
                    &source.name.name,
                    value.span(),
                ));
            }

            Ok(mode_enabled)
        }
    }
}

fn preview_features_guardrail(args: &HashMap<&str, ValueValidator>) -> Result<(), DatamodelError> {
    args.get(PREVIEW_FEATURES_KEY)
        .map(|val| -> Result<_, _> { Ok((val.as_array().to_str_vec()?, val.span())) })
        .transpose()?
        .filter(|(feats, _span)| !feats.is_empty())
        .map(|(_, span)| {
            Err(DatamodelError::new_connector_error(
        "Preview features are only supported in the generator block. Please move this field to the generator block.",
        span,
    ))
        })
        .unwrap_or(Ok(()))
}
