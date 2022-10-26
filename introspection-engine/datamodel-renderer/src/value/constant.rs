use std::{borrow::Cow, fmt};

use once_cell::sync::Lazy;
use regex::Regex;

/// A constant value. Should be used if a value has certain naming
/// standards.
#[derive(Debug)]
pub struct Constant<T: fmt::Display>(T);

impl<'a> Clone for Constant<&'a str> {
    fn clone(&self) -> Self {
        Constant(self.0)
    }
}

impl<'a> Copy for Constant<&'a str> {}

impl<'a> AsRef<str> for Constant<&'a str> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> AsRef<str> for Constant<Cow<'a, str>> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

/// Thrown if a constant cannot be cleanly created with the given
/// input value.
#[derive(Debug)]
pub enum ConstantNameValidationError<'a> {
    /// Constant was invalid but could be sanitized.
    WasSanitized {
        /// A sanitized value to be used as a valid constant.
        sanitized: Constant<Cow<'a, str>>,
    },
    /// The given value was empty and cannot be used as a constant.
    OriginalEmpty,
    /// Constant was invalid impossible to sanitize as something that
    /// is valid in the PSL.
    SanitizedEmpty,
}

impl<'a, T: fmt::Display + 'a> Constant<T> {
    pub(crate) fn new_no_validate(value: T) -> Self {
        Self(value)
    }

    pub(crate) fn boxed(self) -> Constant<Box<dyn fmt::Display + 'a>> {
        Constant(Box::new(self.0) as Box<dyn fmt::Display + 'a>)
    }

    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

impl<'a> Constant<Cow<'a, str>> {
    /// Creates a new constant value. The result has to be checked. If
    /// resulting an error, the error value gives a constant with a
    /// standardized value, and the right side of the tuple is the
    /// actual input value. This input value must be used in a
    /// corresponding `@map`, `@@map` or `map:` declaration, depending
    /// on the position of the constant.
    pub fn new(value: impl Into<Cow<'a, str>>) -> Result<Self, ConstantNameValidationError<'a>> {
        static CONSTANT_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());
        static CONSTANT: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

        let value = value.into();

        if value.is_empty() {
            Err(ConstantNameValidationError::OriginalEmpty)
        } else if CONSTANT_START.is_match(&value) || CONSTANT.is_match(&value) {
            let start_cleaned: String = CONSTANT_START.replace_all(&value, "").parse().unwrap();
            let sanitized: String = CONSTANT.replace_all(start_cleaned.as_str(), "_").parse().unwrap();

            if !sanitized.is_empty() {
                let err = ConstantNameValidationError::WasSanitized {
                    sanitized: Self(Cow::Owned(sanitized)),
                };

                Err(err)
            } else {
                Err(ConstantNameValidationError::SanitizedEmpty)
            }
        } else {
            Ok(Self(value))
        }
    }
}

impl<T> fmt::Display for Constant<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
