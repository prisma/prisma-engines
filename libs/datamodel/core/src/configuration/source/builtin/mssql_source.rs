#![cfg(feature = "mssql")]

use crate::configuration::*;
use datamodel_connector::{Connector, ExampleConnector};

pub const MSSQL_SOURCE_NAME: &str = "sqlserver";

pub struct MSSqlSource {
    pub(super) name: String,
    pub(super) url: StringFromEnvVar,
    pub(super) documentation: Option<String>,
}

impl Source for MSSqlSource {
    fn connector_type(&self) -> &str {
        MSSQL_SOURCE_NAME
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

    fn connector(&self) -> Box<dyn Connector> {
        Box::new(ExampleConnector::mssql())
    }
}
