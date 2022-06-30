use migration_connector::DatabaseSchema;
use sql_schema_describer::{self as sql, walkers::SqlSchemaExt, SqlSchema};

#[derive(Default, Debug)]
pub(crate) struct SqlDatabaseSchema {
    pub(crate) describer_schema: SqlSchema,
    /// A _sorted_ array of column ids with prisma-level defaults.
    pub(crate) prisma_level_defaults: Vec<sql::ColumnId>,
}

impl SqlDatabaseSchema {
    pub(crate) fn from_erased(erased: DatabaseSchema) -> Box<Self> {
        erased.downcast()
    }

    pub(crate) fn walk<I>(&self, id: I) -> sql::Walker<'_, I> {
        self.describer_schema.walk(id)
    }
}

impl From<SqlSchema> for SqlDatabaseSchema {
    fn from(describer_schema: SqlSchema) -> Self {
        SqlDatabaseSchema {
            describer_schema,
            ..Default::default()
        }
    }
}

impl From<SqlDatabaseSchema> for DatabaseSchema {
    fn from(s: SqlDatabaseSchema) -> Self {
        DatabaseSchema::new(s)
    }
}

impl SqlSchemaExt for SqlDatabaseSchema {
    fn table_walker<'a>(&'a self, name: &str) -> Option<sql_schema_describer::walkers::TableWalker<'a>> {
        self.describer_schema.table_walker(name)
    }

    fn view_walker_at(&self, index: usize) -> sql_schema_describer::walkers::ViewWalker<'_> {
        self.describer_schema.view_walker_at(index)
    }

    fn udt_walker_at(&self, index: usize) -> sql_schema_describer::walkers::UserDefinedTypeWalker<'_> {
        self.describer_schema.udt_walker_at(index)
    }
}
