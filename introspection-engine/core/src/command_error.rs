use datamodel::error::ErrorCollection;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandError {
    /// When there are no models or enums detected.
    #[error("The introspected database was empty: {0} .")]
    IntrospectionResultEmpty(String),
    /// When the input datamodel was invalid.
    #[error("The provided input datamodel was invalid: {0} .")]
    InputSchemaInvalid(ErrorCollection),
    #[error("Generic error. (error: {0})")]
    Generic(anyhow::Error),
}
