use crate::{Source, StringFromEnvVar};
use datamodel_connector::Connector;

pub struct SimpleSource {
    pub connector_type: String,
    pub name: String,
    pub url: StringFromEnvVar,
    pub documentation: Option<String>,
    pub connector: Box<dyn Connector>,
}

impl Source for SimpleSource {
    fn connector_type(&self) -> &str {
        &self.connector_type
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

    fn connector(&self) -> &Box<dyn Connector> {
        &self.connector
    }
}
