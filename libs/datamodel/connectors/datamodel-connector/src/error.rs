use crate::datamodel::ast::Span;
use colored::Colorize;
use regex::internal::Input;
use thiserror::Error;

#[rustfmt::skip]
/// Enum for different errors which can happen during parsing or validation.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConnectorError {
    #[error("Argument \"{}\" is missing.", argument_name)]
    ArgumentNotFound {
        argument_name: string,
        span: datamodel::ast::Span,
    },

    #[error(
        "Attribute \"@{}\" takes {} arguments, but received {}.",
        directive_name,
        required_count,
        given_count
    )]
    ArgumentCountMissmatch {
        directive_name: String,
        required_count: usize,
        given_count: usize,
        span: datamodel::ast::Span,
    },

    #[error(
    "Native type \"{}\" is not supported for {} connector.",
    native_type,
    connector
    )]
    TypeNotSupported {
        native_type: String,
        connector: String,
        span: datamodel::ast::Span,
    },
}

#[rustfmt::skip]
impl ConnectorError {
    pub fn new_argument_not_found_error(argument_name: string, span: datamodel::ast::Span) -> ConnectorError {
        ConnectorError::ArgumentNotFound {
            argument_name: String::from(argument_name),
            span,
        }
    }

    pub fn new_type_not_supported_error(native_type: string, connector: string, span: datamodel::ast::Span) -> ConnectorError {
        ConnectorError::TypeNotSupported {
            native_type: String::from(native_type),
            connector: String::from(connector),
            span,
        }
    }

    pub fn new_argument_count_mismatch_error(
        directive_name: string,
        required_count: usize,
        given_count: usize,
        span: datamodel::ast::Span,
    ) -> ConnectorError {
        ConnectorError::ArgumentCountMissmatch {
            directive_name: String::from(directive_name),
            required_count,
            given_count,
            span,
        }
    }

    pub fn span(&self) -> datamodel::ast::Span {
        match self {
            ConnectorError::ArgumentNotFound { span, .. } => *span,
            ConnectorError::ArgumentCountMissmatch { span, .. } => *span,
            ConnectorError::TypeNotSupported { span, .. } => *span,
        }
    }

    pub fn description(&self) -> String {
        format!("{}", self)
    }
}
