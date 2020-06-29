use super::{
    builtin_datasource_providers::{MySqlDatasourceProvider, PostgresDatasourceProvider, SqliteDatasourceProvider},
    datasource_provider::DatasourceProvider,
};
use crate::common::arguments::Arguments;
use crate::common::value_validator::ValueListValidator;
use crate::error::{DatamodelError, ErrorCollection};
use crate::StringFromEnvVar;
use crate::{ast, Datasource};
use datamodel_connector::{BuiltinConnectors, Connector};

/// Helper struct to load and validate source configuration blocks.
pub struct SourceLoader {
    source_definitions: Vec<Box<dyn DatasourceProvider>>,
}

impl SourceLoader {
    /// Creates a new, empty source loader.
    pub fn new() -> Self {
        Self {
            source_definitions: get_builtin_datasource_providers(),
        }
    }

    /// Loads all source config blocks form the given AST,
    /// and returns a Source instance for each.
    pub fn load_sources(
        &self,
        ast_schema: &ast::SchemaAst,
        ignore_datasource_urls: bool,
    ) -> Result<Vec<Datasource>, ErrorCollection> {
        let mut sources = vec![];
        let mut errors = ErrorCollection::new();

        for src in &ast_schema.sources() {
            match self.load_source(&src, ignore_datasource_urls) {
                Ok(loaded_src) => sources.push(loaded_src),
                // Lift error to source.
                Err(DatamodelError::ArgumentNotFound { argument_name, span }) => errors.push(
                    DatamodelError::new_source_argument_not_found_error(&argument_name, &src.name.name, span),
                ),
                Err(err) => errors.push(err),
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(sources)
        }
    }

    /// Internal: Loads a single source from a source config block in the datamodel.
    fn load_source(
        &self,
        ast_source: &ast::SourceConfig,
        ignore_datasource_urls: bool,
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
        let (env_var_for_url, url) = match url_args.as_str_from_env() {
            _ if ignore_datasource_urls => {
                // glorious hack. ask marcus
                (None, format!("{}://", providers.first().unwrap()))
            }
            Ok((env_var, url)) => (env_var, url.trim().to_owned()),
            Err(err) => return Err(err),
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
            .map(|sd| {
                let url_check_result = sd.can_handle_url(source_name, &url).map_err(|err_msg| {
                    DatamodelError::new_source_validation_error(&err_msg, source_name, url_args.span())
                });
                url_check_result.map(|_| sd)
            })
            .collect();

        let combined_connector: Box<dyn Connector> = {
            let connectors = all_datasource_providers.iter().map(|sd| sd.connector()).collect();
            BuiltinConnectors::combined(connectors)
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
        Box::new(MsSqlSourceDefinition::new()),
    ]
}
