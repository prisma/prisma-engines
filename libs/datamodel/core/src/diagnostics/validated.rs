use crate::{ast::reformat::MissingField, diagnostics::DatamodelWarning, Configuration, Datamodel};

#[derive(Debug, PartialEq, Clone)]
pub struct Validated<T> {
    pub subject: T,
    pub warnings: Vec<DatamodelWarning>,
}

pub type ValidatedDatamodel = Validated<Datamodel>;
pub type ValidatedConfiguration = Validated<Configuration>;
pub type ValidatedMissingFields = Validated<Vec<MissingField>>;
