use super::{Datasource, Generator};
use crate::{
    PreviewFeature,
    datamodel_connector::RelationMode,
    diagnostics::{DatamodelError, Diagnostics},
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
}
