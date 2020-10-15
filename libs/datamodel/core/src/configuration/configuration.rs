use super::{Datasource, Generator};
use crate::diagnostics::{DatamodelError, Diagnostics};

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
}
