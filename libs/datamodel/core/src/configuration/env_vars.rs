use serde::Serialize;

/// Either an env var or a string literal.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringFromEnvVar {
    /// Contains the name of env var if the value was read from one.
    pub from_env_var: Option<String>,
    /// Contains the string literal, when it was directly in the parsed schema.
    pub value: Option<String>,
}

impl StringFromEnvVar {
    pub fn new_from_env_var(env_var_name: String) -> StringFromEnvVar {
        StringFromEnvVar {
            from_env_var: Some(env_var_name),
            value: None,
        }
    }

    pub fn new_literal(value: String) -> StringFromEnvVar {
        StringFromEnvVar {
            from_env_var: None,
            value: Some(value),
        }
    }

    /// Returns the name of the env var, if env var.
    pub fn as_env_var(&self) -> Option<&str> {
        self.from_env_var.as_deref()
    }

    /// Returns the contents of the string literal, if applicable.
    pub fn as_literal(&self) -> Option<&str> {
        self.value.as_deref()
    }
}
