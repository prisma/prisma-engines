use super::{Datasource, Generator};
use crate::{
    common::preview_features::PreviewFeature,
    diagnostics::{DatamodelError, Diagnostics},
};

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

    /// Returns true if PlanetScale mode is enabled
    pub fn planet_scale_mode(&self) -> bool {
        self.datasources
            .first()
            .map(|source| source.planet_scale_mode)
            .unwrap_or(false)
    }

    pub fn preview_features(&self) -> impl Iterator<Item = &PreviewFeature> {
        self.generators
            .iter()
            .flat_map(|generator| generator.preview_features.iter())
    }

    pub fn resolve_datasource_urls_from_env(&mut self, url_overrides: &[(String, String)]) -> Result<(), Diagnostics> {
        for datasource in &mut self.datasources {
            if let Some((_, url)) = url_overrides.iter().find(|(name, _url)| name == &datasource.name) {
                datasource.url.value = Some(url.clone());
                datasource.url.from_env_var = None;
            }

            if datasource.url.from_env_var.is_some() && datasource.url.value.is_none() {
                datasource.url.value = Some(datasource.load_url()?);
            }
        }

        Ok(())
    }
}
