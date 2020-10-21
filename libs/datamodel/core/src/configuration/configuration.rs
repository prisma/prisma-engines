use super::{Datasource, Generator};
use crate::diagnostics::{DatamodelError, Diagnostics};
use crate::preview_features::PreviewFeatures;

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

    pub fn preview_features(&self) -> impl Iterator<Item = &str> {
        self.generators
            .iter()
            .flat_map(|generator| generator.preview_features().iter().map(|feat| feat.as_str()))
    }
}
