use serde::{ser::SerializeStruct, Serialize};

/// Either an env var or a string literal.
#[derive(Clone, Debug, PartialEq)]
pub enum StringFromEnvVar {
    /// Contains the name of env var if the value was read from one.
    FromEnvVar(String),
    /// Contains the string literal, when it was directly in the parsed schema.
    Literal(String),
}

impl Serialize for StringFromEnvVar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("StringFromEnvVar", 2)?;

        match dbg!(self) {
            StringFromEnvVar::FromEnvVar(var) => {
                s.serialize_field("fromEnvVar", var)?;
                s.serialize_field("value", &Option::<String>::None)?;
            }
            StringFromEnvVar::Literal(val) => {
                s.serialize_field("fromEnvVar", &Option::<String>::None)?;
                s.serialize_field("value", val)?;
            }
        }

        s.end()
    }
}

impl StringFromEnvVar {
    pub fn new_from_env_var(env_var_name: String) -> StringFromEnvVar {
        StringFromEnvVar::FromEnvVar(env_var_name)
    }

    pub fn new_literal(value: String) -> StringFromEnvVar {
        StringFromEnvVar::Literal(value)
    }

    /// Returns the name of the env var, if env var.
    pub fn as_env_var(&self) -> Option<&str> {
        match self {
            StringFromEnvVar::FromEnvVar(var_name) => Some(var_name),
            _ => None,
        }
    }

    /// Returns the contents of the string literal, if applicable.
    pub fn as_literal(&self) -> Option<&str> {
        match self {
            StringFromEnvVar::Literal(value) => Some(value),
            _ => None,
        }
    }
}
