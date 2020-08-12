use crate::scalars::ScalarType;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub struct ConnectorError {
    details: String,
}

impl ConnectorError {
    pub fn new(msg: &str) -> ConnectorError {
        ConnectorError {
            details: msg.to_string(),
        }
    }

    pub fn new_type_name_unknown_error(native_type: &str, connector: &str) -> ConnectorError {
        ConnectorError {
            details: format!(
                "Native type {} is not supported for {} connector.",
                String::from(native_type),
                String::from(connector)
            ),
        }
    }

    pub fn new_argument_count_mismatch_error(
        native_type: &str,
        required_count: usize,
        given_count: usize,
    ) -> ConnectorError {
        ConnectorError {
            details: format!(
                "Native type {} takes {} arguments, but received {}.",
                String::from(native_type),
                required_count,
                given_count
            ),
        }
    }

    pub fn new_duplicate_directive_error(directive_name: &str) -> ConnectorError {
        ConnectorError {
            details: format!("Attribute @{} is defined twice.", String::from(directive_name)),
        }
    }

    pub fn new_incompatible_native_type_error(
        native_type: &str,
        field_type: ScalarType,
        expected_type: ScalarType,
    ) -> ConnectorError {
        ConnectorError {
            details: format!(
                "Native type {} is not compatible with declared field type {}, expected field type {}.",
                String::from(native_type),
                field_type.to_string(),
                expected_type.to_string()
            ),
        }
    }

    pub fn new_connector_not_supported_for_native_types(connector: &str) -> ConnectorError {
        ConnectorError {
            details: format!(
                "Native types are not supported with {} connector.",
                String::from(connector)
            ),
        }
    }
}

impl fmt::Display for ConnectorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}
