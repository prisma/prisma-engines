use super::builtin_datasource_providers::MsSqlDatasourceProvider;
use super::{
    super::helpers::*,
    builtin_datasource_providers::{MySqlDatasourceProvider, PostgresDatasourceProvider, SqliteDatasourceProvider},
    datasource_provider::DatasourceProvider,
};
use crate::ast::Span;
use crate::configuration::StringFromEnvVar;
use crate::diagnostics::{DatamodelError, DatamodelWarning, Diagnostics, ValidatedDatasource, ValidatedDatasources};
use crate::{ast, Datasource};
use datamodel_connector::{CombinedConnector, Connector};

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
        ignore_datasource_urls: bool,
        datasource_url_overrides: Vec<(String, String)>,
    ) -> Result<ValidatedDatasources, Diagnostics> {
        let mut sources = vec![];
        let mut diagnostics = Diagnostics::new();

        for src in &ast_schema.sources() {
            match self.lift_datasource(&src, ignore_datasource_urls, &datasource_url_overrides) {
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
        ignore_datasource_urls: bool,
        datasource_url_overrides: &[(String, String)],
    ) -> Result<ValidatedDatasource, Diagnostics> {
        let source_name = &ast_source.name.name;
        let mut args = Arguments::new(&ast_source.properties, ast_source.span);
        let mut diagnostics = Diagnostics::new();

        let provider_arg = args.arg("provider")?;

        if provider_arg.is_from_env() {
            return Err(diagnostics.merge_error(DatamodelError::new_functional_evaluation_error(
                &"A datasource must not use the env() function in the provider argument.".to_string(),
                ast_source.span,
            )));
        }

        let providers = provider_arg.as_array().to_str_vec()?;

        if provider_arg.is_array() {
            diagnostics.push_warning(DatamodelWarning::new_deprecated_provider_array_warning(
                provider_arg.span(),
            ))
        }

        if providers.is_empty() {
            return Err(diagnostics.merge_error(DatamodelError::new_source_validation_error(
                "The provider argument in a datasource must not be empty",
                source_name,
                provider_arg.span(),
            )));
        }

        let url_arg = args.arg(URL_KEY)?;
        let override_url = datasource_url_overrides
            .iter()
            .find(|x| &x.0 == source_name)
            .map(|x| &x.1);

        let url = match (url_arg.as_str_from_env(), override_url) {
            (Err(err), _)
                if ignore_datasource_urls && err.description().contains("Expected a String value, but received") =>
            {
                return Err(diagnostics.merge_error(err));
            }
            (_, _) if ignore_datasource_urls => {
                // glorious hack. ask marcus
                StringFromEnvVar {
                    from_env_var: None,
                    value: format!("{}://", providers.first().unwrap()),
                }
            }
            (_, Some(url)) => {
                debug!("overwriting datasource `{}` with url '{}'", &source_name, &url);
                StringFromEnvVar {
                    from_env_var: None,
                    value: url.to_owned(),
                }
            }
            (Ok((env_var, url)), _) => StringFromEnvVar {
                from_env_var: env_var,
                value: url.trim().to_owned(),
            },
            (Err(err), _) => {
                return Err(diagnostics.merge_error(err));
            }
        };

        validate_datasource_url(&url, source_name, &url_arg)?;

        let shadow_database_url_arg = args.optional_arg(SHADOW_DATABASE_URL_KEY);

        let shadow_database_url = if let Some(shadow_database_url_arg) = shadow_database_url_arg {
            let shadow_database_url = match (shadow_database_url_arg.as_str_from_env(), override_url) {
                (Err(err), _)
                    if ignore_datasource_urls
                        && err.description().contains("Expected a String value, but received") =>
                {
                    return Err(diagnostics.merge_error(err));
                }
                (_, _) if ignore_datasource_urls => {
                    // glorious hack. ask marcus
                    StringFromEnvVar {
                        from_env_var: None,
                        value: format!("{}://", providers.first().unwrap()),
                    }
                }
                (_, Some(url)) => {
                    debug!(
                        "overwriting datasource `{}` shadow database url with url '{}'",
                        &source_name, &url
                    );
                    StringFromEnvVar {
                        from_env_var: None,
                        value: url.to_owned(),
                    }
                }
                (Ok((env_var, url)), _) => StringFromEnvVar {
                    from_env_var: env_var,
                    value: url.trim().to_owned(),
                },
                (Err(err), _) => {
                    return Err(diagnostics.merge_error(err));
                }
            };

            validate_datasource_url(&shadow_database_url, source_name, &url_arg)?;

            if url.value == shadow_database_url.value {
                return Err(
                    diagnostics.merge_error(DatamodelError::new_shadow_database_is_same_as_main_url_error(
                        source_name.clone(),
                        shadow_database_url_arg.span(),
                    )),
                );
            }

            Some(shadow_database_url)
        } else {
            None
        };

        preview_features_guardrail(&mut args)?;

        let documentation = ast_source.documentation.as_ref().map(|comment| comment.text.clone());

        let all_datasource_providers: Vec<_> = providers
            .iter()
            .filter_map(|provider| self.get_datasource_provider(&provider))
            .collect();

        if all_datasource_providers.is_empty() {
            return Err(
                diagnostics.merge_error(DatamodelError::new_datasource_provider_not_known_error(
                    &providers.join(","),
                    provider_arg.span(),
                )),
            );
        }

        let validated_providers: Vec<_> = all_datasource_providers
            .iter()
            .map(|provider| {
                let url_check_result = provider.can_handle_url(source_name, &url).map_err(|err_msg| {
                    DatamodelError::new_source_validation_error(&err_msg, source_name, url_arg.span())
                });
                url_check_result.map(|_| provider)
            })
            .collect();

        let combined_connector: Box<dyn Connector> = {
            let connectors = all_datasource_providers.iter().map(|sd| sd.connector()).collect();
            Box::new(CombinedConnector::new(connectors))
        };

        // The first provider that can handle the URL is used to construct the Datasource.
        // If no provider can handle it, return the first error.
        let (successes, errors): (Vec<_>, Vec<_>) = validated_providers.into_iter().partition(|result| result.is_ok());

        if let Some(first_provider) = successes.into_iter().next() {
            let first_successful_provider = first_provider?;

            Ok(ValidatedDatasource {
                subject: Datasource {
                    name: source_name.to_string(),
                    provider: providers,
                    active_provider: first_successful_provider.canonical_name().to_string(),
                    url,
                    documentation,
                    combined_connector,
                    active_connector: first_successful_provider.connector(),
                    shadow_database_url,
                },
                warnings: diagnostics.warnings,
            })
        } else {
            Err(diagnostics.merge_error(errors.into_iter().next().unwrap().err().unwrap()))
        }
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
    ]
}

fn preview_features_guardrail(args: &mut Arguments) -> Result<(), DatamodelError> {
    let preview_features_arg = args.arg(PREVIEW_FEATURES_KEY);
    let (preview_features, span) = match preview_features_arg.ok() {
        Some(x) => (x.as_array().to_str_vec()?, x.span()),
        None => (Vec::new(), Span::empty()),
    };

    if preview_features.is_empty() {
        return Ok(());
    }

    Err(DatamodelError::new_connector_error(
        "Preview features are only supported in the generator block. Please move this field to the generator block.",
        span,
    ))
}

fn validate_datasource_url(
    url: &StringFromEnvVar,
    source_name: &str,
    url_arg: &ValueValidator,
) -> Result<(), DatamodelError> {
    if !url.value.is_empty() {
        return Ok(());
    }

    let suffix = match &url.from_env_var {
        Some(env_var_name) => format!(
            " The environment variable `{}` resolved to an empty string.",
            env_var_name
        ),
        None => "".to_owned(),
    };

    let msg = format!(
        "You must provide a nonempty URL for the datasource `{}`.{}",
        source_name, &suffix
    );

    Err(DatamodelError::new_source_validation_error(
        &msg,
        source_name,
        url_arg.span(),
    ))
}
