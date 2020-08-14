use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandError {
    /// When there are no models or enums detected.
    #[error("The introspected database was empty: {0} .")]
    IntrospectionResultEmpty(String),
    /// When there was a bad datamodel as part of the input.
    #[error("{0}")]
    ReceivedBadDatamodel(String),
    #[error("Generic error. (error: {0})")]
    Generic(anyhow::Error),
}
