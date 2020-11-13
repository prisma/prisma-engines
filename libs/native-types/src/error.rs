use thiserror::Error;

#[derive(Debug, Error)]
pub enum NativeTypeError {
    #[error(
        "Type `{}` takes `{}` optional arguments, but received `{}`.",
        r#type,
        required,
        given
    )]
    OptionalArgumentCountMismatch {
        r#type: String,
        required: usize,
        given: usize,
    },
    #[error("The given {} type `{}` was invalid. Please check the syntax.", database, given)]
    InvalidType { given: String, database: String },
    #[error("Invalid {} type parameter input: `{}`.", database, given)]
    InvalidParameter {
        expected: String,
        given: String,
        database: String,
    },
}

impl NativeTypeError {
    /// The parsed type had either too little or too many arguments.
    pub fn optional_argument_count(r#type: impl ToString, required: usize, given: usize) -> Self {
        Self::OptionalArgumentCountMismatch {
            r#type: r#type.to_string(),
            required,
            given,
        }
    }

    /// The parsed type was not recognized.
    pub fn invalid_type(r#type: impl ToString, database: impl ToString) -> Self {
        Self::InvalidType {
            given: r#type.to_string(),
            database: database.to_string(),
        }
    }

    /// The parsed parameter was invalid.
    pub fn invalid_parameter(param: impl ToString, expected: impl ToString, database: impl ToString) -> Self {
        Self::InvalidParameter {
            expected: expected.to_string(),
            given: param.to_string(),
            database: database.to_string(),
        }
    }
}
