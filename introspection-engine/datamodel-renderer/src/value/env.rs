use std::fmt;

use psl::StringFromEnvVar;

use crate::Text;

/// A value that can optionally be fetched from an environment
/// variable.
#[derive(Debug, Clone, Copy)]
pub enum Env<'a> {
    /// Represents `env("VAR")`, where `var` is the tuple value. The
    /// value is fetched from an env var of the same name.
    FromVar(Text<&'a str>),
    /// Value directly written to the file, not using an env var.
    Value(Text<&'a str>),
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
