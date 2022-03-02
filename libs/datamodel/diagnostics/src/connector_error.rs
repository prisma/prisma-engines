use crate::{DatamodelError, Span};
use thiserror::Error;

pub struct ConnectorErrorFactory {
    native_type: String,
    connector: String,
}

impl ConnectorErrorFactory {
    pub fn new(native_type: String, connector: String) -> Self {
        ConnectorErrorFactory { native_type, connector }
    }

    pub fn new_scale_larger_than_precision_error(self, span: Span) -> DatamodelError {
        DatamodelError::new(
            ErrorKind::ScaleLargerThanPrecisionError {
                native_type: self.native_type,
                connector_name: self.connector,
            }
            .to_string()
            .into(),
            span,
        )
    }

    pub fn new_incompatible_native_type_with_index(self, message: &str, span: Span) -> DatamodelError {
        DatamodelError::new(
            ErrorKind::IncompatibleNativeTypeWithIndexAttribute {
                native_type: self.native_type,
                connector_name: self.connector,
                message: String::from(message),
            }
            .to_string()
            .into(),
            span,
        )
    }

    pub fn new_incompatible_native_type_with_unique(self, message: &str, span: Span) -> DatamodelError {
        DatamodelError::new(
            ErrorKind::IncompatibleNativeTypeWithUniqueAttribute {
                native_type: self.native_type,
                connector_name: self.connector,
                message: String::from(message),
            }
            .to_string()
            .into(),
            span,
        )
    }

    pub fn new_incompatible_native_type_with_id(self, message: &str, span: Span) -> DatamodelError {
        DatamodelError::new(
            ErrorKind::IncompatibleNativeTypeWithIdAttribute {
                native_type: self.native_type,
                connector_name: self.connector,
                message: String::from(message),
            }
            .to_string()
            .into(),
            span,
        )
    }

    pub fn new_incompatible_sequential_type_with_static_default_value_error(self, span: Span) -> DatamodelError {
        DatamodelError::new(
            ErrorKind::IncompatibleSequentialTypeWithStaticDefaultValue {
                native_type: self.native_type,
                connector_name: self.connector,
            }
            .to_string()
            .into(),
            span,
        )
    }

    pub fn new_argument_m_out_of_range_error(self, message: &str, span: Span) -> DatamodelError {
        DatamodelError::new(
            ErrorKind::ArgumentOutOfRangeError {
                native_type: self.native_type,
                connector_name: self.connector,
                message: String::from(message),
            }
            .to_string()
            .into(),
            span,
        )
    }

    pub fn native_type_name_unknown(self, span: Span) -> DatamodelError {
        DatamodelError::new(
            ErrorKind::NativeTypeNameUnknown {
                native_type: self.native_type,
                connector_name: self.connector,
            }
            .to_string()
            .into(),
            span,
        )
    }
}

#[derive(Debug, Error)]
enum ErrorKind {
    #[error("Native type {} is not supported for {} connector.", native_type, connector_name)]
    NativeTypeNameUnknown {
        native_type: String,
        connector_name: String,
    },

    #[error("Native type {} cannot be unique in {}.{}", native_type, connector_name, message)]
    IncompatibleNativeTypeWithUniqueAttribute {
        native_type: String,
        connector_name: String,
        message: String,
    },

    #[error(
        "Native type {} of {} cannot be used on a field that is `@id` or `@@id`.{}",
        native_type,
        connector_name,
        message
    )]
    IncompatibleNativeTypeWithIdAttribute {
        native_type: String,
        connector_name: String,
        message: String,
    },

    #[error(
        "You cannot define an index on fields with Native type {} of {}.{}",
        native_type,
        connector_name,
        message
    )]
    IncompatibleNativeTypeWithIndexAttribute {
        native_type: String,
        connector_name: String,
        message: String,
    },

    #[error(
        "The scale must not be larger than the precision for the {} native type in {}.",
        native_type,
        connector_name
    )]
    ScaleLargerThanPrecisionError {
        native_type: String,
        connector_name: String,
    },

    #[error(
        "Sequential native type {} of {} must not have a static default value.",
        native_type,
        connector_name
    )]
    IncompatibleSequentialTypeWithStaticDefaultValue {
        native_type: String,
        connector_name: String,
    },

    #[error(
        "Argument M is out of range for Native type {} of {}: {}",
        native_type,
        connector_name,
        message
    )]
    ArgumentOutOfRangeError {
        native_type: String,
        connector_name: String,
        message: String,
    },
}
