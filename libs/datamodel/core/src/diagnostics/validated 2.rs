use crate::ast::reformat::MissingField;
use crate::diagnostics::DatamodelWarning;
use crate::{Configuration, Datamodel, Datasource, Generator};

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
