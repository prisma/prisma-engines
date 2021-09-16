use super::{Datasource, Generator};
use crate::{
    common::preview_features::PreviewFeature,
    diagnostics::{DatamodelError, Diagnostics},
};
use datamodel_connector::ReferentialIntegrity;
use enumflags2::BitFlags;

#[derive(Debug)]
pub struct Configuration {
    pub generators: Vec<Generator>,
    pub datasources: Vec<Datasource>,
}

impl Configuration {
    pub fn validate_that_one_datasource_is_provided(&self) -> Result<(), Diagnostics> {
        if self.datasources.is_empty() {
            Err(DatamodelError::new_validation_error(
                "You defined no datasource. You must define exactly one datasource.",
                crate::ast::Span::new(0, 0),
            )
            .into())
        } else {
            Ok(())
        }
    }

    pub fn referential_integrity(&self) -> Option<ReferentialIntegrity> {
        self.datasources.first().map(|source| source.referential_integrity())
    }

    pub fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.generators
            .iter()
            .fold(BitFlags::empty(), |acc, generator| acc | generator.preview_features())
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
}
