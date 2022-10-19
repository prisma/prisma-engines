use nom::error::Error as NomError;
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Error)]
pub struct TemplatingError {
    ident: String,
    kind: TemplatingErrorKind,
}

impl Display for TemplatingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error parsing ident `{}`: {}", self.ident, self.kind)
    }
}

impl TemplatingError {
    pub fn num_args(ident: &str, expected: usize, got: usize) -> Self {
        Self {
            ident: ident.to_owned(),
            kind: TemplatingErrorKind::NumArgsError { expected, got },
        }
    }

    pub fn unknown_ident(ident: &str) -> Self {
        Self {
            ident: ident.to_owned(),
            kind: TemplatingErrorKind::UnknownIdentError,
        }
    }

    pub fn nom_error(ident: &str, reason: String) -> Self {
        Self {
            ident: ident.to_owned(),
            kind: TemplatingErrorKind::NomError(reason),
        }
    }

    pub fn argument_error(ident: &str, reason: String) -> Self {
        Self {
            ident: ident.to_owned(),
            kind: TemplatingErrorKind::ArgumentError(reason),
        }
    }
}

#[derive(Debug, Error)]
pub enum TemplatingErrorKind {
    #[error("Unexpected number of arguments: Expected (at least) {expected}, got {got}.")]
    NumArgsError { expected: usize, got: usize },

    #[error("Unknown schema interpolation identifier.")]
    UnknownIdentError,

    #[error("Error parsing schema: {0}")]
    NomError(String),

    #[error("Argument error: {0}")]
    ArgumentError(String),
}

impl<T> From<NomError<T>> for TemplatingError
where
    T: Display,
{
    fn from(err: NomError<T>) -> Self {
        Self {
            ident: "Unknown".to_owned(),
            kind: TemplatingErrorKind::NomError(err.to_string()),
        }
    }
}
