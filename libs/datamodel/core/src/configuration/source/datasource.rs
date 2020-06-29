use crate::StringFromEnvVar;
use datamodel_connector::Connector;

pub struct Datasource {
    pub name: String,
    pub connector_type: String,
    pub url: StringFromEnvVar,
    pub documentation: Option<String>,
    pub connector: Box<dyn Connector>,
}

impl Datasource {
    pub fn connector_type(&self) -> &str {
        &self.connector_type
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn url(&self) -> &StringFromEnvVar {
        &self.url
    }

    pub fn set_url(&mut self, url: &str) {
        self.url = StringFromEnvVar {
            from_env_var: None,
            value: url.to_string(),
        };
    }

    pub fn documentation(&self) -> &Option<String> {
        &self.documentation
    }

    pub fn connector(&self) -> &Box<dyn Connector> {
        &self.connector
    }
}
