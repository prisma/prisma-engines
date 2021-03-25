use serde::Serialize;

#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct StringFromEnvVar {
    /// the name of the param this String / EnvVar is for
    #[serde(skip_serializing)]
    pub name: &'static str,
    /// contains the name of env var if the value was read from one
    pub from_env_var: Option<String>,
    pub value: String,
}