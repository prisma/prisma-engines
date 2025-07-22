use user_facing_errors::schema_engine::{ExcessiveNamespaceInExternalTables, MissingNamespaceInExternalTables};

use crate::{ConnectorError, ConnectorResult, Namespaces};

/// Configuration of entities in the schema/database to be included or excluded from an operation.
#[derive(Debug, Default)]
pub struct SchemaFilter {
    /// Tables that shall be considered "externally" managed. As per prisma.config.ts > tables.external.
    /// Prisma will not consider those tables during diffing operations, migration creation, or introspection.
    /// They are still available for querying at runtime.
    pub external_tables: Vec<String>,
}

impl SchemaFilter {
    /// Validate that the schema filter contains correctly qualified table names.
    pub fn validate(&self, namespaces: &Option<Namespaces>) -> ConnectorResult<()> {
        let has_explicit_namespaces = namespaces.is_some();

        for table_name in self.external_tables.iter() {
            if has_explicit_namespaces && !table_name.contains(".") {
                return Err(ConnectorError::user_facing(MissingNamespaceInExternalTables));
            } else if !has_explicit_namespaces && table_name.contains(".") {
                return Err(ConnectorError::user_facing(ExcessiveNamespaceInExternalTables));
            }
        }

        Ok(())
    }
}

impl From<json_rpc::types::SchemaFilter> for SchemaFilter {
    fn from(filter: json_rpc::types::SchemaFilter) -> Self {
        Self {
            external_tables: filter.external_tables,
        }
    }
}

impl From<Option<json_rpc::types::SchemaFilter>> for SchemaFilter {
    fn from(filter: Option<json_rpc::types::SchemaFilter>) -> Self {
        Self {
            external_tables: filter.map(|f| f.external_tables).unwrap_or_default(),
        }
    }
}
