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
}
