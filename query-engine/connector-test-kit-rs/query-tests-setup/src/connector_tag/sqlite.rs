use datamodel_connector::{Connector, ReferentialIntegrity};
use sql_datamodel_connector::SqliteDatamodelConnector;

use super::*;
use crate::SqlDatamodelRenderer;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SqliteConnectorTag {
    capabilities: Vec<ConnectorCapability>,
}

impl ConnectorTagInterface for SqliteConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "sqlite"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(&self, database: &str, _is_ci: bool) -> String {
        let workspace_root = std::env::var("WORKSPACE_ROOT")
            .unwrap_or_else(|_| ".".to_owned())
            .trim_end_matches('/')
            .to_owned();

        format!("file://{}/db/{}.db", workspace_root, database)
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        ("sqlite".to_owned(), None)
    }

    fn is_versioned(&self) -> bool {
        false
    }
}

impl SqliteConnectorTag {
    pub fn new() -> Self {
        Self {
            capabilities: sqlite_capabilities(),
        }
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![Self::new()]
    }
}

fn sqlite_capabilities() -> Vec<ConnectorCapability> {
    let dm_connector = SqliteDatamodelConnector::new(ReferentialIntegrity::default());
    dm_connector.capabilities().to_owned()
}
