use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

/// The type of a parameter given to a database type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NativeTypeParameter {
    /// Number of bytes or characters the type can use.
    Number(u64),
    /// A string literal.
    Literal(Cow<'static, str>),
}

impl NativeTypeParameter {
    /// Creates a new number type parameter.
    pub fn number(value: impl Into<u64>) -> Self {
        Self::Number(value.into())
    }

    /// Creates a new string literal type parameter.
    pub fn literal(value: impl Into<Cow<'static, str>>) -> Self {
        Self::Literal(value.into())
    }
}

impl fmt::Display for NativeTypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{}", n),
            Self::Literal(l) => write!(f, "{}", l),
        }
    }
}

impl From<u8> for NativeTypeParameter {
    fn from(p: u8) -> Self {
        Self::Number(p.into())
    }
}

impl From<u16> for NativeTypeParameter {
    fn from(p: u16) -> Self {
        Self::Number(p.into())
    }
}

impl From<u32> for NativeTypeParameter {
    fn from(p: u32) -> Self {
        Self::Number(p.into())
    }
}

impl From<u64> for NativeTypeParameter {
    fn from(p: u64) -> Self {
        Self::Number(p)
    }
}

impl From<&'static str> for NativeTypeParameter {
    fn from(p: &'static str) -> Self {
        Self::Literal(p.into())
    }
}

impl From<String> for NativeTypeParameter {
    fn from(p: String) -> Self {
        Self::Literal(p.into())
    }
}
