/// Configuration of entities in the schema/database to be included or excluded from an operation.
#[derive(Debug, Default)]
pub struct SchemaFilter {
    /// Tables that shall be considered "externally" managed. As per prisma.config.ts > tables.external.
    /// Prisma will not consider those tables during diffing operations, migration creation, or introspection.
    /// They are still available for querying at runtime.
    pub external_tables: Vec<String>,
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
