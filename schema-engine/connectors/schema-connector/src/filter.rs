use crate::Namespaces;

/// Configuration of entities in the schema/database to be included or excluded from an operation.
#[derive(Debug, Default)]
pub struct SchemaFilter {
    /// Tables that shall be considered "externally" managed. As per prisma.config.ts > tables.external.
    /// Prisma will not consider those tables during diffing operations, migration creation, or introspection.
    /// They are still available for querying at runtime.
    pub external_tables: Vec<String>,

    /// Namespaces that shall be considered.
    pub included_namespaces: Option<Namespaces>,
}

impl SchemaFilter {
    /// Create a SchemaFilter from a json_rpc::types::SchemaFilter and a set of namespaces.
    pub fn from_filter_and_namespaces(
        filter: Option<json_rpc::types::SchemaFilter>,
        namespaces: Option<Namespaces>,
    ) -> Self {
        Self {
            external_tables: filter.map(|f| f.external_tables).unwrap_or_default(),
            included_namespaces: namespaces,
        }
    }

    /// Check if the given table name is in the list of external tables.
    /// `external_tables` can contain fully qualified table names with namespace
    /// (e.g. "auth.user") or just the table name.
    pub fn is_table_external(&self, namespace: Option<&str>, table_name: &str) -> bool {
        if let Some(namespace) = namespace {
            self.external_tables.contains(&format!("{namespace}.{table_name}"))
                || self.external_tables.contains(&table_name.to_string())
        } else {
            self.external_tables.contains(&table_name.to_string())
        }
    }
}

impl From<json_rpc::types::SchemaFilter> for SchemaFilter {
    fn from(filter: json_rpc::types::SchemaFilter) -> Self {
        Self {
            external_tables: filter.external_tables,
            included_namespaces: Default::default(),
        }
    }
}

impl From<Option<json_rpc::types::SchemaFilter>> for SchemaFilter {
    fn from(filter: Option<json_rpc::types::SchemaFilter>) -> Self {
        Self {
            external_tables: filter.map(|f| f.external_tables).unwrap_or_default(),
            included_namespaces: Default::default(),
        }
    }
}
