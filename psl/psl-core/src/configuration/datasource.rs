use schema_ast::ast::WithSpan;

use crate::{
    configuration::StringFromEnvVar,
    datamodel_connector::{Connector, ConnectorCapabilities, RelationMode},
    diagnostics::{DatamodelError, Diagnostics, Span},
    set_config_dir,
};
use std::{any::Any, borrow::Cow, path::Path};

/// a `datasource` from the prisma schema.
pub struct Datasource {
    pub name: String,
    /// Span of the whole datasource block (including `datasource` keyword and braces)
    pub span: Span,
    /// The provider string
    pub provider: String,
    /// The provider that was selected as active from all specified providers
    pub active_provider: &'static str,
    pub url: StringFromEnvVar,
    pub url_span: Span,
    pub direct_url: Option<StringFromEnvVar>,
    pub direct_url_span: Option<Span>,
    pub documentation: Option<String>,
    /// the connector of the active provider
    pub active_connector: &'static dyn Connector,
    /// An optional user-defined shadow database URL.
    pub shadow_database_url: Option<(StringFromEnvVar, Span)>,
    /// In which layer referential actions are handled.
    pub relation_mode: Option<RelationMode>,
    /// _Sorted_ vec of schemas defined in the schemas property.
    pub namespaces: Vec<(String, Span)>,
    pub schemas_span: Option<Span>,
    pub connector_data: DatasourceConnectorData,
}

pub enum UrlValidationError {
    EmptyUrlValue,
    EmptyEnvValue(String),
    NoEnvValue(String),
    NoUrlOrEnv,
}

#[derive(Default)]
pub struct DatasourceConnectorData {
    data: Option<Box<dyn Any + Send + Sync + 'static>>,
}

impl DatasourceConnectorData {
    pub fn new(data: Box<dyn Any + Send + Sync + 'static>) -> Self {
        Self { data: Some(data) }
    }

    #[track_caller]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.data.as_ref().map(|data| data.downcast_ref().unwrap())
    }
}

impl std::fmt::Debug for Datasource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Datasource")
            .field("name", &self.name)
            .field("provider", &self.provider)
            .field("active_provider", &self.active_provider)
            .field("url", &"<url>")
            .field("documentation", &self.documentation)
            .field("active_connector", &&"...")
            .field("shadow_database_url", &"<shadow_database_url>")
            .field("relation_mode", &self.relation_mode)
            .field("namespaces", &self.namespaces)
            .finish()
    }
}

impl Datasource {
    /// Extract connector-specific constructs. The type parameter must be the right one.
    #[track_caller]
    pub fn downcast_connector_data<T: 'static>(&self) -> Option<&T> {
        self.connector_data.downcast_ref()
    }

    pub(crate) fn has_schema(&self, name: &str) -> bool {
        self.namespaces.binary_search_by_key(&name, |(s, _)| s).is_ok()
    }

    pub fn capabilities(&self) -> ConnectorCapabilities {
        self.active_connector.capabilities()
    }

    /// The applicable relation mode for this datasource.
    #[allow(clippy::or_fun_call)] // not applicable in this case
    pub fn relation_mode(&self) -> RelationMode {
        self.relation_mode
            .unwrap_or(self.active_connector.default_relation_mode())
    }

    /// Load the database URL, validating it and resolving env vars in the
    /// process. Also see `load_url_with_config_dir()`.
    pub fn load_url<F>(&self, env: F) -> Result<String, Diagnostics>
    where
        F: Fn(&str) -> Option<String>,
    {
        let url = self.load_url_no_validation(env)?;

        self.active_connector.validate_url(&url).map_err(|err_str| {
            let err_str = if url.starts_with("prisma") {
                let s = indoc::formatdoc! {"
                    {err_str}

                    To use a URL with protocol `prisma://`, you need to either enable Accelerate or the Data Proxy.
                    Enable Accelerate via `prisma generate --accelerate` or the Data Proxy via `prisma generate --data-proxy.`

                    More information about Data Proxy: https://pris.ly/d/data-proxy
                "};

                Cow::from(s)
            } else {
                Cow::from(err_str)
            };

            DatamodelError::new_source_validation_error(&format!("the URL {}", &err_str), &self.name, self.url_span)
        })?;

        Ok(url)
    }

    /// Load the database URL, without validating it and resolve env vars in the
    /// process.
    pub fn load_url_no_validation<F>(&self, env: F) -> Result<String, Diagnostics>
    where
        F: Fn(&str) -> Option<String>,
    {
        from_url(&self.url, env).map_err(|err| match err {
                UrlValidationError::EmptyUrlValue => {
                    let msg = "You must provide a nonempty URL";
                    DatamodelError::new_source_validation_error(msg, &self.name, self.url_span).into()
                }
                UrlValidationError::EmptyEnvValue(env_var) => {
                    DatamodelError::new_source_validation_error(
                        &format!("You must provide a nonempty URL. The environment variable `{env_var}` resolved to an empty string."),
                        &self.name,
                        self.url_span,
                    )
                    .into()
                }
                UrlValidationError::NoEnvValue(env_var) => {
                    DatamodelError::new_environment_functional_evaluation_error(env_var, self.url_span).into()
                }
                UrlValidationError::NoUrlOrEnv => unreachable!("Missing url in datasource"),
        })
    }

    /// Load the direct database URL, validating it and resolving env vars in the
    /// process. If there is no `directUrl` passed, it will default to `load_url()`.
    ///
    pub fn load_direct_url<F>(&self, env: F) -> Result<String, Diagnostics>
    where
        F: Fn(&str) -> Option<String>,
    {
        let validate_direct_url = |(url, span)| {
            let handle_err = |err| match err {
                UrlValidationError::EmptyUrlValue => {
                    let msg = "You must provide a nonempty direct URL";
                    Err(DatamodelError::new_source_validation_error(msg, &self.name, span).into())
                }
                UrlValidationError::EmptyEnvValue(env_var) => {
                    let msg = format!(
                        "You must provide a nonempty direct URL. The environment variable `{env_var}` resolved to an empty string."
                    );

                    Err(DatamodelError::new_source_validation_error(&msg, &self.name, span).into())
                }
                UrlValidationError::NoEnvValue(env_var) => {
                    let e = DatamodelError::new_environment_functional_evaluation_error(env_var, span);
                    Err(e.into())
                }
                UrlValidationError::NoUrlOrEnv => self.load_url(&env),
            };

            let url = from_url(&url, &env).map_or_else(handle_err, Result::Ok)?;

            if url.starts_with("prisma://") {
                let msg = "You must provide a direct URL that points directly to the database. Using `prisma` in URL scheme is not allowed.";
                let e = DatamodelError::new_source_validation_error(msg, &self.name, span);

                Err(e.into())
            } else {
                Ok(url)
            }
        };

        self.direct_url
            .clone()
            .and_then(|url| self.direct_url_span.map(|span| (url, span)))
            .map_or_else(|| self.load_url(&env), validate_direct_url)
    }

    /// Same as `load_url()`, with the following difference.
    ///
    /// By default we treat relative paths (in the connection string and
    /// datasource url value) as relative to the CWD. This does not work in all
    /// cases, so we need a way to prefix these relative paths with a
    /// config_dir.
    ///
    /// This is, at the time of this writing (2021-05-05), only used in the
    /// context of Node-API integration.
    ///
    /// P.S. Don't forget to add new parameters here if needed!
    pub fn load_url_with_config_dir<F>(&self, config_dir: &Path, env: F) -> Result<String, Diagnostics>
    where
        F: Fn(&str) -> Option<String>,
    {
        //CHECKUP
        let url = self.load_url(env)?;
        let url = set_config_dir(self.active_connector.flavour(), config_dir, &url);

        Ok(url.into_owned())
    }

    /// Load the shadow database URL, validating it and resolving env vars in the process.
    pub fn load_shadow_database_url(&self) -> Result<Option<String>, Diagnostics> {
        let (url, url_span) = match self
            .shadow_database_url
            .as_ref()
            .map(|(url, span)| (&url.value, &url.from_env_var, span))
        {
            None => return Ok(None),
            Some((Some(lit), _, span)) => (lit.clone(), span),
            Some((None, Some(env_var), span)) => match std::env::var(env_var) {
                // We explicitly ignore empty and missing env vars, because the same schema (with the same env function) has to be usable for dev and deployment alike.
                Ok(var) if var.trim().is_empty() => return Ok(None),
                Err(_) => return Ok(None),

                Ok(var) => (var, span),
            },
            Some((None, None, _span)) => unreachable!("Missing url in datasource"),
        };

        if !url.trim().is_empty() {
            self.active_connector.validate_url(&url).map_err(|err_str| {
                DatamodelError::new_source_validation_error(
                    &format!("the shadow database URL {}", &err_str),
                    &self.name,
                    *url_span,
                )
            })?;
        }

        Ok(Some(url))
    }

    // Validation for property existence
    pub fn provider_defined(&self) -> bool {
        !self.provider.is_empty()
    }

    pub fn url_defined(&self) -> bool {
        self.url_span.end > self.url_span.start
    }

    pub fn direct_url_defined(&self) -> bool {
        self.direct_url.is_some()
    }

    pub fn shadow_url_defined(&self) -> bool {
        self.shadow_database_url.is_some()
    }

    pub fn relation_mode_defined(&self) -> bool {
        self.relation_mode.is_some()
    }

    pub fn schemas_defined(&self) -> bool {
        self.schemas_span.is_some()
    }
}

impl WithSpan for Datasource {
    fn span(&self) -> Span {
        self.span
    }
}

pub(crate) fn from_url<F>(url: &StringFromEnvVar, env: F) -> Result<String, UrlValidationError>
where
    F: Fn(&str) -> Option<String>,
{
    let url = match (&url.value, &url.from_env_var) {
        (Some(lit), _) if lit.trim().is_empty() => {
            return Err(UrlValidationError::EmptyUrlValue);
        }
        (Some(lit), _) => lit.clone(),
        (None, Some(env_var)) => match env(env_var) {
            Some(var) if var.trim().is_empty() => {
                return Err(UrlValidationError::EmptyEnvValue(env_var.clone()));
            }
            Some(var) => var,
            None => return Err(UrlValidationError::NoEnvValue(env_var.clone())),
        },
        (None, None) => return Err(UrlValidationError::NoUrlOrEnv),
    };

    Ok(url)
}
