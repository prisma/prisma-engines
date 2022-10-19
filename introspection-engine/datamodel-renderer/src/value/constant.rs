use std::{borrow::Cow, fmt};

use once_cell::sync::Lazy;
use regex::Regex;

/// A constant value. Should be used if a value has certain naming
/// standards.
#[derive(Debug)]
pub struct Constant<'a>(Cow<'a, str>);

/// Thrown if a constant cannot be cleanly created with the given
/// input value.
#[derive(Debug)]
pub enum ConstantNameValidationError<'a> {
    /// Constant was invalid but could be sanitized.
    WasSanitized {
        /// A sanitized value to be used as a valid constant.
        sanitized: Constant<'a>,
        /// The original value to be used in the corresponding `@@map`
        /// or `@map` attributes, or the `map:` argument.
        original: &'a str,
    },
    /// The given value was empty and cannot be used as a constant.
    OriginalEmpty,
    /// Constant was invalid impossible to sanitize as something that
    /// is valid in the PSL.
    SanitizedEmpty,
}

impl<'a> Constant<'a> {
    /// Creates a new constant value. The result has to be checked. If
    /// resulting an error, the error value gives a constant with a
    /// standardized value, and the right side of the tuple is the
    /// actual input value. This input value must be used in a
    /// corresponding `@map`, `@@map` or `map:` declaration, depending
    /// on the position of the constant.
    pub fn new(value: &'a str) -> Result<Self, ConstantNameValidationError<'a>> {
        static CONSTANT_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());
        static CONSTANT: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

        if value.is_empty() {
            Err(ConstantNameValidationError::OriginalEmpty)
        } else if CONSTANT_START.is_match(value) || CONSTANT.is_match(value) {
            let start_cleaned: String = CONSTANT_START.replace_all(value, "").parse().unwrap();
            let sanitized: String = CONSTANT.replace_all(start_cleaned.as_str(), "_").parse().unwrap();

            if !sanitized.is_empty() {
                let err = ConstantNameValidationError::WasSanitized {
                    sanitized: Self(Cow::Owned(sanitized)),
                    original: value,
                };

                Err(err)
            } else {
                Err(ConstantNameValidationError::SanitizedEmpty)
            }
        } else {
            Ok(Self(Cow::Borrowed(value)))
        }
    }

    pub(crate) fn new_no_validate(value: &'a str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl<'a> fmt::Display for Constant<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
