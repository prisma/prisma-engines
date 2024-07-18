use super::{Datasource, Generator};
use crate::{
    datamodel_connector::RelationMode,
    diagnostics::{DatamodelError, Diagnostics},
    PreviewFeature,
};
use enumflags2::BitFlags;

#[derive(Debug, Default)]
pub struct Configuration {
    pub generators: Vec<Generator>,
    pub datasources: Vec<Datasource>,
    pub warnings: Vec<diagnostics::DatamodelWarning>,
}

impl Configuration {
    pub fn new(
        generators: Vec<Generator>,
        datasources: Vec<Datasource>,
        warnings: Vec<diagnostics::DatamodelWarning>,
    ) -> Self {
        Self {
            generators,
            datasources,
            warnings,
        }
    }

    pub fn extend(&mut self, other: Configuration) {
        self.generators.extend(other.generators);
        self.datasources.extend(other.datasources);
        self.warnings.extend(other.warnings);
    }

    pub fn validate_that_one_datasource_is_provided(&self) -> Result<(), Diagnostics> {
        if self.datasources.is_empty() {
            Err(DatamodelError::new_validation_error(
                "You defined no datasource. You must define exactly one datasource.",
                schema_ast::ast::Span::new(0, 0, diagnostics::FileId::ZERO),
            )
            .into())
        } else {
            Ok(())
        }
    }

    pub fn relation_mode(&self) -> Option<RelationMode> {
        self.datasources.first().map(|source| source.relation_mode())
    }

    pub fn max_identifier_length(&self) -> usize {
        self.datasources
            .first()
            .map(|source| source.active_connector.max_identifier_length())
            .unwrap_or(usize::MAX)
    }

    pub fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.generators.iter().fold(BitFlags::empty(), |acc, generator| {
            acc | generator.preview_features.unwrap_or_default()
        })
    }

    /// Resolve datasource url for query engine.
    ///
    /// The main interesting thing here is we want to ignore any error that may arise from resolving
    /// direct_url.
    pub fn resolve_datasource_urls_query_engine<F>(
        &mut self,
        url_overrides: &[(String, String)],
        env: F,
        ignore_env_errors: bool,
    ) -> Result<(), Diagnostics>
    where
        F: Fn(&str) -> Option<String> + Copy,
    {
        for datasource in &mut self.datasources {
            if let Some((_, url)) = url_overrides.iter().find(|(name, _url)| name == &datasource.name) {
                datasource.url.value = Some(url.clone());
                datasource.url.from_env_var = None;
            }

            if datasource.url.from_env_var.is_some() && datasource.url.value.is_none() {
                datasource.url.value = match datasource.load_url(env) {
                    Ok(url) => Some(url),
                    Err(_) if ignore_env_errors => None,
                    Err(error) => return Err(error),
                };
            }

            if let Some(direct_url) = &datasource.direct_url {
                let result = match super::from_url(direct_url, env) {
                    Err(_) => None, // ignore errors because we don't really need direct_url in QE
                    Ok(res) => Some(res),
                };

                datasource.direct_url = Some(crate::StringFromEnvVar {
                    from_env_var: direct_url.from_env_var.clone(),
                    value: result,
                });
            }
        }

        Ok(())
    }

    /// Resolve datasource URL's for getConfig.
    /// The main reason this exists is:
    ///   - we want to error if we can't resolve direct_url
    ///   - we want to skip validation for url IF we have a direct_url
    ///
    /// For that last bit, we only do this currently because our validation errors on URL's starting
    /// with 'prisma://'. We would ideally like to do the other validations and ignore in this case.
    pub fn resolve_datasource_urls_prisma_fmt<F>(
        &mut self,
        url_overrides: &[(String, String)],
        env: F,
    ) -> Result<(), Diagnostics>
    where
        F: Fn(&str) -> Option<String> + Copy,
    {
        for datasource in &mut self.datasources {
            if let Some((_, url)) = url_overrides.iter().find(|(name, _url)| name == &datasource.name) {
                datasource.url.value = Some(url.clone());
                datasource.url.from_env_var = None;
            }

            let mut has_direct_url = false;

            if let (Some(direct_url), Some(span)) = (&datasource.direct_url, &datasource.direct_url_span) {
                let result = match super::from_url(direct_url, env) {
                        Err(err) => {
                            match err {
                        super::UrlValidationError::EmptyUrlValue => {
                            let msg = "You must provide a nonempty direct URL";
                            Err(DatamodelError::new_source_validation_error(msg, &datasource.name, *span))
                        }
                        super::UrlValidationError::EmptyEnvValue(env_var) => {
                            Err(DatamodelError::new_source_validation_error(
                                &format!(
                                    "You must provide a nonempty direct URL. The environment variable `{env_var}` resolved to an empty string."
                                ),
                                &datasource.name,
                                *span,
                            ))
                        },
                        super::UrlValidationError::NoEnvValue(env_var) => {
                            Err(DatamodelError::new_environment_functional_evaluation_error(
                                env_var,
                                *span,
                            ))
                        },
                        super::UrlValidationError::NoUrlOrEnv => {
                          Ok(None)
                        },
                    }
                        },
                        Ok(res) => Ok(Some(res)),
                    }?;

                has_direct_url = true;

                datasource.direct_url = Some(crate::StringFromEnvVar {
                    from_env_var: direct_url.from_env_var.clone(),
                    value: result,
                });
            }

            // We probably just need to improve validation, especially around allowing 'prisma://'
            // urls.
            if datasource.url.from_env_var.is_some() && datasource.url.value.is_none() {
                if has_direct_url {
                    datasource.url.value = Some(datasource.load_url_no_validation(env)?);
                } else {
                    datasource.url.value = Some(datasource.load_url(env)?);
                }
            }
        }

        Ok(())
    }

    pub fn first_datasource(&self) -> &Datasource {
        self.datasources.first().expect("Expected a datasource to exist.")
    }
}
