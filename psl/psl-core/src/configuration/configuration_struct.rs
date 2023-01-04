use super::{Datasource, Generator};
use crate::{
    datamodel_connector::RelationMode,
    diagnostics::{DatamodelError, Diagnostics},
    PreviewFeature,
};
use enumflags2::BitFlags;

#[derive(Debug)]
pub struct Configuration {
    pub generators: Vec<Generator>,
    pub datasources: Vec<Datasource>,
    pub warnings: Vec<diagnostics::DatamodelWarning>,
}

impl Configuration {
    pub fn validate_that_one_datasource_is_provided(&self) -> Result<(), Diagnostics> {
        if self.datasources.is_empty() {
            Err(DatamodelError::new_validation_error(
                "You defined no datasource. You must define exactly one datasource.",
                schema_ast::ast::Span::new(0, 0),
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

    pub fn resolve_datasource_urls_from_env<F>(
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

            if datasource.url.from_env_var.is_some() && datasource.url.value.is_none() {
                datasource.url.value = Some(datasource.load_url(env)?);
            }
        }

        Ok(())
    }

    pub fn resolve_direct_datasource_urls_from_env<F>(
        &mut self,
        url_overrides: &[(String, String)],
        env: F,
    ) -> Result<(), Diagnostics>
    where
        F: Fn(&str) -> Option<String> + Copy,
    {
        for datasource in &mut self.datasources {
            let url_override = url_overrides.iter().find(|(name, _url)| name == &datasource.name);

            match (
                url_override,
                datasource
                    .direct_url
                    .as_ref()
                    .and_then(|direct_url| direct_url.from_env_var.as_ref()),
                &datasource.url.from_env_var,
                &datasource.url.value,
            ) {
                // We can't grab it from anywhere, so we give up.
                (None, None, None, None) => {}
                // URL overrides trump everything else.
                (Some((_, url)), _, _, _) => {
                    datasource.url.value = Some(url.clone());
                    datasource.url.from_env_var = None;
                }
                // We have a directUrl, so we can/should override.
                (_, Some(_), _, _) => {
                    datasource.url.value = Some(datasource.load_direct_url(env)?);
                }
                // We already have an URL, so we ignore the env var.
                (_, _, _, Some(_)) => {}
                // We don't have an URL, but we have an env var, so we resolve it.
                (_, _, Some(_), None) => {
                    datasource.url.value = Some(datasource.load_url(env)?);
                }
            }
        }

        Ok(())
    }
}
