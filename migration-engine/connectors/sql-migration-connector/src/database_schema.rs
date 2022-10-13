use migration_connector::DatabaseSchema;
use sql_schema_describer::{self as sql, SqlSchema};

#[derive(Debug)]
pub(crate) struct SqlDatabaseSchema {
    pub(crate) describer_schema: SqlSchema,
    /// A _sorted_ array of column ids with prisma-level defaults.
    pub(crate) prisma_level_defaults: Vec<sql::ColumnId>,
    /// Namespaces considered relevant for this particular schema.
    pub(crate) relevant_namespaces: RelevantNamespaces,
}

#[derive(Debug)]
pub(crate) enum RelevantNamespaces {
    All,
    NotApplicable,
    Some(sql::NamespaceId, Vec<sql::NamespaceId>),
}

impl SqlDatabaseSchema {
    pub(crate) fn from_erased(erased: DatabaseSchema) -> Box<Self> {
        erased.downcast()
    }

    pub(crate) fn walk<I>(&self, id: I) -> sql::Walker<'_, I> {
        self.describer_schema.walk(id)
    }
}

impl From<SqlDatabaseSchema> for DatabaseSchema {
    fn from(s: SqlDatabaseSchema) -> Self {
        DatabaseSchema::new(s)
    }
}
