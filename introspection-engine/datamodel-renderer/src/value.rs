//! Common types needed in the configuration and datamodel.

mod array;
mod constant;
mod documentation;
mod env;
mod function;
mod text;

pub use array::Array;
use base64::display::Base64Display;
pub use constant::{Constant, ConstantNameValidationError};
pub use documentation::Documentation;
pub use env::Env;
pub use function::{Function, FunctionParam};
pub use text::Text;

use std::fmt;

/// A PSL value representation.
pub enum Value<'a> {
    /// A string value, quoted and escaped accordingly.
    Text(Text<&'a str>),
    /// A byte value, quoted and base64-encoded.
    Bytes(Text<Base64Display<'a>>),
    /// A constant value without quoting.
    Constant(Constant<Box<dyn fmt::Display + 'a>>),
    /// An array of values.
    Array(Array<Value<'a>>),
    /// A function has a name, and optionally named parameters.
    Function(Function<'a>),
    /// A value can be read from the environment.
    Env(Env<'a>),
}

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Text(t) => f.debug_tuple("Text").field(t).finish(),
            Value::Constant(val) => {
                write!(f, "Constant({val})")
            }
            Value::Array(ary) => f.debug_tuple("Array").field(ary).finish(),
            Value::Function(fun) => f.debug_tuple("Function").field(fun).finish(),
            Value::Env(e) => f.debug_tuple("Env").field(e).finish(),
            Value::Bytes(Text(b)) => write!(f, "Bytes({b})"),
        }
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        let display = Base64Display::with_config(bytes, base64::STANDARD);
        Self::Bytes(Text(display))
    }
}

impl<'a> From<Text<&'a str>> for Value<'a> {
    fn from(t: Text<&'a str>) -> Self {
        Self::Text(t)
    }
}

impl<'a> From<Array<Value<'a>>> for Value<'a> {
    fn from(t: Array<Value<'a>>) -> Self {
        Self::Array(t)
    }
}

impl<'a> From<Function<'a>> for Value<'a> {
    fn from(t: Function<'a>) -> Self {
        Self::Function(t)
    }
}

impl<'a> From<Env<'a>> for Value<'a> {
    fn from(t: Env<'a>) -> Self {
        Self::Env(t)
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Text(val) => {
                val.fmt(f)?;
            }
            Value::Constant(val) => {
                val.fmt(f)?;
            }
            Value::Array(array) => {
                array.fmt(f)?;
            }
            Value::Function(fun) => {
                fun.fmt(f)?;
            }
            Value::Env(env) => {
                env.fmt(f)?;
            }
            Value::Bytes(val) => {
                write!(f, "{val}")?;
            }
        }

        Ok(())
    }
}
