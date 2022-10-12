use std::{borrow::Cow, fmt};

use psl::{common::preview_features::PreviewFeature, StringFromEnvVar};

/// Represents a string value in the PSL.
#[derive(Debug, Clone, Copy)]
pub struct Text<'a>(pub &'a str);

impl<'a> fmt::Display for Text<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&psl::schema_ast::string_literal(self.0), f)
    }
}

/// Adding slashes before things.
#[derive(Debug)]
pub enum Commented<'a> {
    /// We use two slashes for disabled rows during introspection.
    DisabledRows(&'a str),
    /// A documentation block on top of an item in the PSL.
    Documentation(&'a str),
}

impl<'a> fmt::Display for Commented<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Commented::DisabledRows(text) => {
                for line in text.split('\n') {
                    f.write_str("//")?;

                    if !line.is_empty() {
                        f.write_str(" ")?;
                    }

                    f.write_str(line)?;
                    f.write_str("\n")?;
                }
            }
            Commented::Documentation(text) => {
                for line in text.split('\n') {
                    f.write_str("///")?;

                    if !line.is_empty() {
                        f.write_str(" ")?;
                    }

                    f.write_str(line)?;
                    f.write_str("\n")?;
                }
            }
        }

        Ok(())
    }
}

/// A value that can optionally be fetched from an environment
/// variable.
#[derive(Debug, Clone, Copy)]
pub enum Env<'a> {
    /// Represents `env("VAR")`, where `var` is the tuple value. The
    /// value is fetched from an env var of the same name.
    FromVar(Text<'a>),
    /// Value directly written to the file, not using an env var.
    Value(Text<'a>),
}

impl<'a> Env<'a> {
    /// Represents `env("VAR")`, where `var` is the tuple value. The
    /// value is fetched from an env var of the same name.
    pub fn variable(var: &'a str) -> Self {
        Self::FromVar(Text(var))
    }

    /// Value directly written to the file, not using an env var.
    pub fn value(val: &'a str) -> Self {
        Self::Value(Text(val))
    }
}

impl<'a> From<&'a StringFromEnvVar> for Env<'a> {
    fn from(other: &'a StringFromEnvVar) -> Self {
        match (other.as_env_var(), other.as_literal()) {
            (Some(var), _) => Self::variable(var),
            (_, Some(val)) => Self::value(val),
            _ => unreachable!(),
        }
    }
}

impl<'a> fmt::Display for Env<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Env::FromVar(var) => {
                write!(f, "env({var})")
            }
            Env::Value(val) => val.fmt(f),
        }
    }
}

/// Represents a function parameter in the PSL.
#[derive(Debug)]
pub enum FunctionParam<'a> {
    /// key: value
    KeyValue(&'a str, Value<'a>),
    /// value (only)
    OnlyValue(Value<'a>),
}

impl<'a> From<Value<'a>> for FunctionParam<'a> {
    fn from(v: Value<'a>) -> Self {
        Self::OnlyValue(v)
    }
}

impl<'a, T> From<(&'a str, T)> for FunctionParam<'a>
where
    T: Into<Value<'a>>,
{
    fn from(kv: (&'a str, T)) -> Self {
        Self::KeyValue(kv.0, kv.1.into())
    }
}

impl<'a> fmt::Display for FunctionParam<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionParam::KeyValue(k, v) => {
                write!(f, "{k}: {v}")
            }
            FunctionParam::OnlyValue(v) => v.fmt(f),
        }
    }
}

/// Represents a function value in the PSL.
#[derive(Debug)]
pub struct Function<'a> {
    name: Cow<'a, str>,
    params: Vec<FunctionParam<'a>>,
}

impl<'a> Function<'a> {
    /// Creates a plain function with no parameters.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            params: Default::default(),
        }
    }

    /// Add a new parameter to the function. If no parameters are
    /// added, the parentheses are not rendered.
    pub fn push_param(&mut self, param: impl Into<FunctionParam<'a>>) {
        self.params.push(param.into());
    }
}

impl<'a> fmt::Display for Function<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;

        if !self.params.is_empty() {
            f.write_str("(")?;
        }

        for (i, param) in self.params.iter().enumerate() {
            param.fmt(f)?;

            if i < self.params.len() {
                f.write_str(", ")?;
            }
        }

        if !self.params.is_empty() {
            f.write_str(")")?;
        }

        Ok(())
    }
}

/// An array of values.
#[derive(Debug, Default)]
pub struct Array<'a>(pub(crate) Vec<Value<'a>>);

impl<'a> Array<'a> {
    /// Returns `true` if the array contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the length of the array.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Add a new value to the end of the array.
    pub fn push(&mut self, val: impl Into<Value<'a>>) {
        self.0.push(val.into());
    }
}

impl<'a> fmt::Display for Array<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;

        for (i, val) in self.0.iter().enumerate() {
            val.fmt(f)?;

            if i < self.0.len() - 1 {
                f.write_str(", ")?;
            }
        }

        f.write_str("]")?;

        Ok(())
    }
}

/// A PSL value representation.
#[derive(Debug)]
pub enum Value<'a> {
    /// A string value, quoted and escaped accordingly.
    Text(Text<'a>),
    /// A constant value without quoting.
    Constant(&'a str),
    /// An array of values.
    Array(Array<'a>),
    /// A function has a name, and optionally named parameters.
    Function(Function<'a>),
    /// A value can be read from the environment.
    Env(Env<'a>),
    /// Prisma preview feature
    Feature(PreviewFeature),
}

impl<'a> From<Text<'a>> for Value<'a> {
    fn from(t: Text<'a>) -> Self {
        Self::Text(t)
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(t: &'a str) -> Self {
        Self::Constant(t)
    }
}

impl<'a> From<Array<'a>> for Value<'a> {
    fn from(t: Array<'a>) -> Self {
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

impl From<PreviewFeature> for Value<'_> {
    fn from(feat: PreviewFeature) -> Self {
        Self::Feature(feat)
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Text(val) => {
                val.fmt(f)?;
            }
            Value::Constant(val) => {
                f.write_str(val)?;
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
            Value::Feature(feat) => {
                f.write_str("\"")?;
                feat.fmt(f)?;
                f.write_str("\"")?;
            }
        }

        Ok(())
    }
}
