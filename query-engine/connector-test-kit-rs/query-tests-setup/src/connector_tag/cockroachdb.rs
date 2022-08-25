use super::*;
use crate::datamodel_rendering::SqlDatamodelRenderer;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CockroachDbConnectorTag {
    capabilities: Vec<ConnectorCapability>,
}

impl ConnectorTagInterface for CockroachDbConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "cockroachdb"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(&self, database: &str, is_ci: bool, _is_multi_schema: bool) -> String {
        // Use the same database and schema name for CockroachDB - unfortunately CockroachDB
        // can't handle 1 schema per test in a database well at this point in time.
        if is_ci {
            format!("postgresql://prisma@test-db-cockroachdb:26257/{0}?schema={0}", database)
        } else {
            format!("postgresql://prisma@127.0.0.1:26257/{0}?schema={0}", database)
        }
    }

    fn capabilities(&self) -> &[ConnectorCapability] {
        &self.capabilities
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        ("cockroachdb".to_owned(), None)
    }

    fn is_versioned(&self) -> bool {
        false
    }

    fn requires_teardown(&self) -> bool {
        true
    }
}

impl CockroachDbConnectorTag {
    pub fn new() -> Self {
        Self {
            capabilities: cockroachdb_capabilities(),
        }
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![Self::new()]
    }
}

fn cockroachdb_capabilities() -> Vec<ConnectorCapability> {
    sql_datamodel_connector::COCKROACH.capabilities().to_owned()
}
