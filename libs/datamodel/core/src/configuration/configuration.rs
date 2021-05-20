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
    pub fn validate_that_one_datasource_is_provided(self) -> Result<Self, Diagnostics> {
        if self.datasources.is_empty() {
            Err(DatamodelError::new_validation_error(
                "You defined no datasource. You must define exactly one datasource.",
                crate::ast::Span::new(0, 0),
            )
            .into())
        } else {
            Ok(self)
        }
    }

    pub fn preview_features(&self) -> impl Iterator<Item = &PreviewFeature> {
        self.generators
            .iter()
            .flat_map(|generator| generator.preview_features.iter())
    }

    pub fn resolve_datasource_urls_from_env(&mut self) -> Result<(), Diagnostics> {
        for datasource in &mut self.datasources {
            if datasource.url.from_env_var.is_some() && datasource.url.value.is_none() {
                datasource.url.value = Some(datasource.load_url()?);
            }
        }

        Ok(())
    }
}
