use crate::scalars::ScalarType;
use colored::Colorize;
use regex::internal::Input;
use thiserror::Error;

#[rustfmt::skip]
/// Enum for different errors which can happen during parsing or validation.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConnectorError {

    #[error(
        "Native type \"{}\" takes {} arguments, but received {}.",
        native_type,
        required_count,
        given_count
    )]
    ArgumentCountMissmatch {
        native_type: String,
        required_count: usize,
        given_count: usize,
    },

    #[error(
    "Native type \"{}\" is not supported for {} connector.",
    native_type,
    connector
    )]
    UnknownTypeName {
        native_type: String,
        connector: String,
    },

    #[error(
    "Attribute \"@{}\" is defined twice.",
    directive_name
    )]
    DuplicateDirective {
        directive_name: String,
    },

    #[error(
    "Native type \"{}\" is not compatible with declared field type {}, expected field type {}.",
    native_type,
    field_type,
    expected_type
    )]
    IncompatibleType {
        native_type: String,
        field_type: String,
        expected_type: String,
    },

}

#[rustfmt::skip]
impl ConnectorError {

    pub fn new_type_name_unknown_error(native_type: string, connector: string) -> ConnectorError {
        ConnectorError::UnknownTypeName {
            native_type: String::from(native_type),
            connector: String::from(connector),
        }
    }

    pub fn new_argument_count_mismatch_error(
        native_type: string,
        required_count: usize,
        given_count: usize,
    ) -> ConnectorError {
        ConnectorError::ArgumentCountMissmatch {
            native_type: String::from(native_type),
            required_count,
            given_count,
        }
    }

    pub fn new_duplicate_directive_error(directive_name: &str) -> ConnectorError {
        ConnectorError::DuplicateDirective {
            directive_name: String::from(directive_name),
        }
    }

    pub fn new_incompatible_native_type_error(native_type: &str, field_type: ScalarType, expected_type: ScalarType) -> ConnectorError {
        ConnectorError::IncompatibleType {
            native_type: String::from(native_type),
            field_type: field_type.to_string(),
            expected_type: expected_type.to_string(),
        }
    }

}
