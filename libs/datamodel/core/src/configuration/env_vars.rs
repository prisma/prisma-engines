use crate::ast;
use serde::Serialize;

#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct StringFromEnvVar {
    /// the name of the param this String / EnvVar is fore
    #[serde(skip_serializing)]
    pub name: &'static str,
    /// contains the name of env var if the value was read from one
    pub from_env_var: Option<String>,
    pub value: String,
}

impl StringFromEnvVar {
    pub fn to_arg(&self) -> ast::Argument {
        match self.from_env_var {
            Some(ref env_var) => {
                let values = vec![ast::Expression::StringValue(env_var.to_string(), ast::Span::empty())];
                ast::Argument::new_function("url", self.name, values)
            }
            None => ast::Argument::new_string("url", &self.value),
        }
    }
}
