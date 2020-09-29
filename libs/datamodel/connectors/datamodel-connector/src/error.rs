use thiserror::Error;

#[derive(Debug, Error)]
#[error("{}", kind)]
pub struct ConnectorError {
    /// The error information for internal use.
    pub kind: ErrorKind,
}

impl ConnectorError {
    pub fn from_kind(kind: ErrorKind) -> Self {
        ConnectorError { kind }
    }

    pub fn new_argument_count_mismatch_error(
        native_type: &str,
        required_count: usize,
        given_count: usize,
    ) -> ConnectorError {
        ConnectorError::from_kind(ErrorKind::ArgumentCountMisMatchError {
            native_type: String::from(native_type),
            required_count,
            given_count,
        })
    }
}

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("Native types are not supported with {} connector", connector_name)]
    ConnectorNotSupportedForNativeTypes { connector_name: String },

    #[error(
        "The prefix {} is invalid. It must be equal to the name of an existing datasource e.g. {}. Did you mean to use {}?",
        given_prefix,
        expected_prefix,
        suggestion
    )]
    InvalidPrefixForNativeTypes {
        given_prefix: String,
        expected_prefix: String,
        suggestion: String,
    },

    #[error(
        "Native type {} is not compatible with declared field type {}, expected field type {}.",
        native_type,
        field_type,
        expected_type
    )]
    IncompatibleNativeType {
        native_type: String,
        field_type: String,
        expected_type: String,
    },

    #[error("Attribute @{} is defined twice.", directive_name)]
    DuplicateDirectiveError { directive_name: String },

    #[error("Native type {} is not supported for {} connector.", native_type, connector_name)]
    NativeTypeNameUnknown {
        native_type: String,
        connector_name: String,
    },

    #[error(
        "Native type {} takes {} arguments, but received {}.",
        native_type,
        required_count,
        given_count
    )]
    ArgumentCountMisMatchError {
        native_type: String,
        required_count: usize,
        given_count: usize,
    },

    #[error(
    "Native types can only be used if the corresponding feature flag is enabled. Please add this field in your datasource block: `previewFeatures = [\"nativeTypes\"]`"
    )]
    NativeFlagsPreviewFeatureDisabled,
}
