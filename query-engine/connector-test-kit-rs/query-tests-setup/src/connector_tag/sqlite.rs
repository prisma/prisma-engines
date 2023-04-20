use super::*;
use crate::SqlDatamodelRenderer;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SqliteConnectorTag;

impl ConnectorTagInterface for SqliteConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "sqlite"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(
        &self,
        database: &str,
        _is_ci: bool,
        _is_multi_schema: bool,
        _: Option<&'static str>,
    ) -> String {
        let workspace_root = std::env::var("WORKSPACE_ROOT")
            .unwrap_or_else(|_| ".".to_owned())
            .trim_end_matches('/')
            .to_owned();

        format!("file://{workspace_root}/db/{database}.db")
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::SQLITE.capabilities()
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
        Self
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![Self::new()]
    }
}
