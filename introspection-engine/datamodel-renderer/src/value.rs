pub mod array;
pub mod constant;
pub mod documentation;
pub mod env;
pub mod function;
pub mod text;

use std::fmt;

use self::{
    array::Array,
    constant::{Constant, ConstantNameValidationError},
    env::Env,
    function::Function,
    text::Text,
};

/// A PSL value representation.
#[derive(Debug)]
pub enum Value<'a> {
    /// A string value, quoted and escaped accordingly.
    Text(Text<&'a str>),
    /// A constant value without quoting.
    Constant(Constant<'a>),
    /// An array of values.
    Array(Array<Value<'a>>),
    /// A function has a name, and optionally named parameters.
    Function(Function<'a>),
    /// A value can be read from the environment.
    Env(Env<'a>),
}

impl<'a> From<Text<&'a str>> for Value<'a> {
    fn from(t: Text<&'a str>) -> Self {
        Self::Text(t)
    }
}

impl<'a> TryFrom<&'a str> for Value<'a> {
    type Error = ConstantNameValidationError<'a>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let constant = Constant::new(value)?;

        Ok(Self::Constant(constant))
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
        }

        Ok(())
    }
}
