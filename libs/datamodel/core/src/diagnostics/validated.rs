use crate::{
    ast::reformat::MissingField,
    diagnostics::{DatamodelWarning, Diagnostics},
    Configuration, Datamodel, Datasource, Generator, ValidationFeature,
};
use enumflags2::BitFlags;

pub trait ParseDatamodel {
    fn parse_datamodel_with_flags<T>(&self, flags: T) -> Result<Validated<Datamodel>, Diagnostics>
    where
        T: Into<BitFlags<ValidationFeature>>;

    fn parse_datamodel(&self) -> Result<Validated<Datamodel>, Diagnostics> {
        self.parse_datamodel_with_flags(BitFlags::empty())
    }
}

pub trait ParseConfiguration {
    fn parse_config_with_flags<T>(&self, flags: T) -> Result<Validated<Configuration>, Diagnostics>
    where
        T: Into<BitFlags<ValidationFeature>>;

    fn parse_config(&self) -> Result<Validated<Configuration>, Diagnostics> {
        self.parse_config_with_flags(BitFlags::empty())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Validated<T> {
    pub subject: T,
    pub warnings: Vec<DatamodelWarning>,
}

pub type ValidatedDatamodel = Validated<Datamodel>;
pub type ValidatedConfiguration = Validated<Configuration>;
pub type ValidatedDatasource = Validated<Datasource>;
pub type ValidatedDatasources = Validated<Vec<Datasource>>;
pub type ValidatedGenerator = Validated<Generator>;
pub type ValidatedGenerators = Validated<Vec<Generator>>;
pub type ValidatedMissingFields = Validated<Vec<MissingField>>;
