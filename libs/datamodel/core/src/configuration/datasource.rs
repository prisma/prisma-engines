use crate::{
    ast::Span,
    configuration::StringFromEnvVar,
    diagnostics::{DatamodelError, Diagnostics},
};
use datamodel_connector::{Connector, ConnectorCapabilities};
use std::path::Path;

/// a `datasource` from the prisma schema.
pub struct Datasource {
    pub name: String,
    /// The provider string
    pub provider: String,
    /// the provider that was selected as active from all specified providers
    pub active_provider: String,
    pub url: StringFromEnvVar,
    pub url_span: Span,
    pub documentation: Option<String>,
    /// the connector of the active provider
    pub active_connector: Box<dyn Connector>,
    /// An optional user-defined shadow database URL.
    pub shadow_database_url: Option<(StringFromEnvVar, Span)>,
    /// Whether planetScaleMode = true was provided
    pub planet_scale_mode: bool,
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
            .field("shadow_database_url", &self.shadow_database_url)
            .finish()
    }
}

impl Datasource {
    pub fn capabilities(&self) -> ConnectorCapabilities {
        let capabilities = self.active_connector.capabilities().to_owned();
        ConnectorCapabilities::new(capabilities)
    }

    /// Load the database URL, validating it and resolving env vars in the
    /// process. Also see `load_url_with_config_dir()`.
    pub fn load_url<F>(&self, env: F) -> Result<String, Diagnostics>
    where
        F: Fn(&str) -> Option<String>,
    {
        let url = match (&self.url.value, &self.url.from_env_var) {
            (Some(lit), _) if lit.trim().is_empty() => {
                let msg = "You must provide a nonempty URL";

                return Err(DatamodelError::new_source_validation_error(msg, &self.name, self.url_span).into());
            }
            (Some(lit), _) => lit.clone(),
            (None, Some(env_var)) => match env(env_var) {
                Some(var) if var.trim().is_empty() => {
                    return Err(DatamodelError::new_source_validation_error(
                        &format!(
                        "You must provide a nonempty URL. The environment variable `{}` resolved to an empty string.",
                        env_var
                    ),
                        &self.name,
                        self.url_span,
                    )
                    .into())
                }
                Some(var) => var,
                None => {
                    return Err(DatamodelError::new_environment_functional_evaluation_error(
                        env_var.to_owned(),
                        self.url_span,
                    )
                    .into())
                }
            },
            (None, None) => unreachable!("Missing url in datasource"),
        };

        self.active_connector.validate_url(&url).map_err(|err_str| {
            DatamodelError::new_source_validation_error(&format!("the URL {}", &err_str), &self.name, self.url_span)
        })?;

        Ok(url)
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
        let url = self.load_url(env)?;
        let url = self.active_connector.set_config_dir(config_dir, &url);

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
}
