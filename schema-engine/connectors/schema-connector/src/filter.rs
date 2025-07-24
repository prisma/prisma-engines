use user_facing_errors::schema_engine::{MissingNamespaceInExternalTables, UnexpectedNamespaceInExternalTables};

use crate::{ConnectorError, ConnectorResult, SchemaDialect};

/// Configuration of entities in the schema/database to be included or excluded from an operation.
#[derive(Debug, Default)]
pub struct SchemaFilter {
    /// Tables that shall be considered "externally" managed. As per prisma.config.ts > tables.external.
    /// Prisma will not consider those tables during diffing operations, migration creation, or introspection.
    /// They are still available for querying at runtime.
    pub external_tables: Vec<String>,
    /// Enums that shall be considered "externally" managed. As per prisma.config.ts > enums.external.
    /// Prisma will not consider those enums during diffing operations, migration creation, or introspection.
    /// They are still available for querying at runtime.
    pub external_enums: Vec<String>,
}

impl SchemaFilter {
    /// Validate that the schema filter contains correctly qualified table and enum names.
    pub fn validate(&self, dialect: &dyn SchemaDialect) -> ConnectorResult<()> {
        let requires_explicit_namespaces = dialect.default_namespace().is_some();

        for table_name in self.external_tables.iter() {
            if requires_explicit_namespaces && !table_name.contains(".") {
                return Err(ConnectorError::user_facing(MissingNamespaceInExternalTables));
            } else if !requires_explicit_namespaces && table_name.contains(".") {
                return Err(ConnectorError::user_facing(UnexpectedNamespaceInExternalTables));
            }
        }

        for enum_name in self.external_enums.iter() {
            if requires_explicit_namespaces && !enum_name.contains(".") {
                return Err(ConnectorError::user_facing(MissingNamespaceInExternalTables));
            } else if !requires_explicit_namespaces && enum_name.contains(".") {
                return Err(ConnectorError::user_facing(UnexpectedNamespaceInExternalTables));
            }
        }

        Ok(())
    }
}

impl From<json_rpc::types::SchemaFilter> for SchemaFilter {
    fn from(filter: json_rpc::types::SchemaFilter) -> Self {
        Self {
            external_tables: filter.external_tables,
            external_enums: filter.external_enums,
        }
    }
}
