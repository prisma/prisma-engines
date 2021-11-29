use crate::DatamodelWarning;

#[derive(Debug, PartialEq, Clone)]
pub struct Validated<T> {
    pub subject: T,
    pub warnings: Vec<DatamodelWarning>,
}
