use crate::{configuration::*, dml, validator::directive::DirectiveValidator};
pub const MYSQL_SOURCE_NAME: &str = "mysql";

pub struct MySqlSource {
    pub(super) name: String,
    pub(super) url: StringFromEnvVar,
    pub(super) documentation: Option<String>,
}

impl Source for MySqlSource {
    fn connector_type(&self) -> &str {
        MYSQL_SOURCE_NAME
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn url(&self) -> &StringFromEnvVar {
        &self.url
    }

    fn set_url(&mut self, url: &str) {
        self.url = StringFromEnvVar {
            from_env_var: None,
            value: url.to_string(),
        };
    }

    fn documentation(&self) -> &Option<String> {
        &self.documentation
    }
}
