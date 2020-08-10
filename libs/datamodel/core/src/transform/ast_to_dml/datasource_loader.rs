#[cfg(feature = "mssql")]
use super::builtin_datasource_providers::MsSqlDatasourceProvider;
use super::{
    super::helpers::*,
    builtin_datasource_providers::{MySqlDatasourceProvider, PostgresDatasourceProvider, SqliteDatasourceProvider},
    datasource_provider::DatasourceProvider,
};
use crate::configuration::StringFromEnvVar;
use crate::error::{DatamodelError, ErrorCollection};
use crate::{ast, Datasource};
use datamodel_connector::{CombinedConnector, Connector};

/// Is responsible for loading and validating Datasources defined in an AST.
pub struct DatasourceLoader {
    source_definitions: Vec<Box<dyn DatasourceProvider>>,
}

impl DatasourceLoader {
    pub fn new() -> Self {
        Self {
            source_definitions: get_builtin_datasource_providers(),
        }
    }

    /// Loads all datasources from the provided schema AST.
    /// - `ignore_datasource_urls`: datasource URLs are not parsed. They are replaced with dummy values.
    /// - `datasource_url_overrides`: datasource URLs are not parsed and overridden with the provided ones.
    pub fn load_sources(
        &self,
        ast_schema: &ast::SchemaAst,
        ignore_datasource_urls: bool,
        datasource_url_overrides: Vec<(String, String)>,
    ) -> Result<Vec<Datasource>, ErrorCollection> {
        let mut sources = vec![];
        let mut errors = ErrorCollection::new();

        for src in &ast_schema.sources() {
            match self.load_source(&src, ignore_datasource_urls, &datasource_url_overrides) {
                Ok(loaded_src) => sources.push(loaded_src),
                // Lift error to source.
                Err(DatamodelError::ArgumentNotFound { argument_name, span }) => errors.push(
                    DatamodelError::new_source_argument_not_found_error(&argument_name, &src.name.name, span),
                ),
                Err(err) => errors.push(err),
            }
        }

        if sources.len() > 1 {
            for src in &ast_schema.sources() {
                errors.push(DatamodelError::new_source_validation_error(
                    &format!("You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet."),
                    &src.name.name,
                    src.span.clone(),
                ));
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(sources)
        }
    }

    fn load_source(
        &self,
        ast_source: &ast::SourceConfig,
        ignore_datasource_urls: bool,
        datasource_url_overrides: &Vec<(String, String)>,
    ) -> Result<Datasource, DatamodelError> {
        let source_name = &ast_source.name.name;
        let mut args = Arguments::new(&ast_source.properties, ast_source.span);

        let provider_arg = args.arg("provider")?;
        if provider_arg.is_from_env() {
            return Err(DatamodelError::new_functional_evaluation_error(
                &format!("A datasource must not use the env() function in the provider argument."),
                ast_source.span,
            ));
        }
        let providers = provider_arg.as_array().to_str_vec()?;

        if providers.is_empty() {
            return Err(DatamodelError::new_source_validation_error(
                "The provider argument in a datasource must not be empty",
                source_name,
                provider_arg.span(),
            ));
        }

        let url_args = args.arg("url")?;
        let override_url = datasource_url_overrides
            .iter()
            .find(|x| &x.0 == source_name)
            .map(|x| &x.1);

        let (env_var_for_url, url) = match (url_args.as_str_from_env(), override_url) {
            (_, _) if ignore_datasource_urls => {
                // glorious hack. ask marcus
                (None, format!("{}://", providers.first().unwrap()))
            }
            (_, Some(url)) => {
                debug!("overwriting datasource `{}` with url '{}'", &source_name, &url);
                (None, url.to_owned())
            }
            (Ok((env_var, url)), _) => (env_var, url.trim().to_owned()),
            (Err(err), _) => return Err(err),
        };

        if url.is_empty() {
            let suffix = match &env_var_for_url {
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
            return Err(DatamodelError::new_source_validation_error(
                &msg,
                source_name,
                url_args.span(),
            ));
        }

        let documentation = ast_source.documentation.clone().map(|comment| comment.text);
        let url = StringFromEnvVar {
            from_env_var: env_var_for_url,
            value: url,
        };

        let all_datasource_providers: Vec<_> = providers
            .iter()
            .filter_map(|provider| self.get_datasource_provider(&provider))
            .collect();

        if all_datasource_providers.is_empty() {
            return Err(DatamodelError::new_datasource_provider_not_known_error(
                &providers.join(","),
                provider_arg.span(),
            ));
        }

        let validated_providers: Vec<_> = all_datasource_providers
            .iter()
            .map(|provider| {
                let url_check_result = provider.can_handle_url(source_name, &url).map_err(|err_msg| {
                    DatamodelError::new_source_validation_error(&err_msg, source_name, url_args.span())
                });
                url_check_result.map(|_| provider)
            })
            .collect();

        let combined_connector: Box<dyn Connector> = {
            let connectors = all_datasource_providers.iter().map(|sd| sd.connector()).collect();
            CombinedConnector::new(connectors)
        };

        // The first provider that can handle the URL is used to construct the Datasource.
        // If no provider can handle it, return the first error.
        let (successes, errors): (Vec<_>, Vec<_>) = validated_providers.into_iter().partition(|result| result.is_ok());
        if !successes.is_empty() {
            let first_successful_provider = successes.into_iter().next().unwrap()?;
            Ok(Datasource {
                name: source_name.to_string(),
                provider: providers,
                active_provider: first_successful_provider.canonical_name().to_string(),
                url,
                documentation: documentation.clone(),
                combined_connector,
                active_connector: first_successful_provider.connector(),
            })
        } else {
            Err(errors.into_iter().next().unwrap().err().unwrap())
        }
    }

    fn get_datasource_provider(&self, provider: &str) -> Option<&Box<dyn DatasourceProvider>> {
        self.source_definitions.iter().find(|sd| sd.is_provider(provider))
    }
}

fn get_builtin_datasource_providers() -> Vec<Box<dyn DatasourceProvider>> {
    vec![
        Box::new(MySqlDatasourceProvider::new()),
        Box::new(PostgresDatasourceProvider::new()),
        Box::new(SqliteDatasourceProvider::new()),
        #[cfg(feature = "mssql")]
        Box::new(MsSqlDatasourceProvider::new()),
    ]
}
