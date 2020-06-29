use crate::StringFromEnvVar;
use datamodel_connector::Connector;

pub struct Datasource {
    pub name: String,
    pub provider: Vec<String>,
    pub active_provider: String,
    pub url: StringFromEnvVar,
    pub documentation: Option<String>,
    pub connector: Box<dyn Connector>,
}

impl Datasource {
    pub fn url(&self) -> &StringFromEnvVar {
        &self.url
    }

    pub fn set_url(&mut self, url: &str) {
        self.url = StringFromEnvVar {
            from_env_var: None,
            value: url.to_string(),
        };
    }
}
