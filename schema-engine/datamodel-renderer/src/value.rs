//! Common types needed in the configuration and datamodel.

mod array;
mod constant;
mod documentation;
mod env;
mod function;
mod text;

pub use array::Array;
pub use constant::Constant;
pub use documentation::Documentation;
pub use env::Env;
pub use function::{Function, FunctionParam};
pub use text::Text;

use crate::{datamodel::IndexOps, Cow};
use base64::display::Base64Display;
use psl::GeneratorConfigValue;
use std::fmt;

/// A PSL value representation.
pub enum Value<'a> {
    /// A string value, quoted and escaped accordingly.
    Text(Text<Cow<'a, str>>),
    /// A byte value, quoted and base64-encoded.
    Bytes(Text<Base64Display<'a, 'static, base64::engine::GeneralPurpose>>),
    /// A constant value without quoting.
    Constant(Cow<'a, str>),
    /// An array of values.
    Array(Array<Value<'a>>),
    /// A function has a name, and optionally named parameters.
    Function(Function<'a>),
    /// A value can be read from the environment.
    Env(Env<'a>),
    /// An index ops definition.
    IndexOps(IndexOps<'a>),
}

impl fmt::Debug for Value<'_> {
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
            Value::IndexOps(ops) => write!(f, "IndexOps({ops})"),
        }
    }
}

impl<'a> From<IndexOps<'a>> for Value<'a> {
    fn from(ops: IndexOps<'a>) -> Self {
        Self::IndexOps(ops)
    }
}

impl<T> From<Constant<T>> for Value<'_>
where
    T: fmt::Display,
{
    fn from(c: Constant<T>) -> Self {
        Self::Constant(c.to_string().into())
    }
}

impl From<Vec<u8>> for Value<'_> {
    fn from(bytes: Vec<u8>) -> Self {
        let display = Base64Display::new(&bytes, &base64::engine::general_purpose::STANDARD).to_string();
        Self::Text(Text::new(display))
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        let display = Base64Display::new(bytes, &base64::engine::general_purpose::STANDARD);
        Self::Bytes(Text(display))
    }
}

impl<'a> From<Text<Cow<'a, str>>> for Value<'a> {
    fn from(t: Text<Cow<'a, str>>) -> Self {
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

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Self::Text(Text(Cow::from(s)))
    }
}

impl<'a> From<Vec<Value<'a>>> for Value<'a> {
    fn from(vec: Vec<Value<'a>>) -> Self {
        Self::Array(vec.into())
    }
}

impl<'a> From<&'a GeneratorConfigValue> for Value<'a> {
    fn from(value: &'a GeneratorConfigValue) -> Self {
        match value {
            GeneratorConfigValue::String(s) => s.as_str().into(),
            GeneratorConfigValue::Array(elements) => elements.iter().map(From::from).collect(),
            GeneratorConfigValue::Env(var_name) => Env::variable(var_name).into(),
        }
    }
}

impl<'a> FromIterator<Value<'a>> for Value<'a> {
    fn from_iter<T: IntoIterator<Item = Value<'a>>>(iter: T) -> Self {
        Self::Array(Array::from(iter.into_iter().collect::<Vec<_>>()))
    }
}

impl fmt::Display for Value<'_> {
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
            Value::IndexOps(ops) => ops.fmt(f)?,
        }

        Ok(())
    }
}
