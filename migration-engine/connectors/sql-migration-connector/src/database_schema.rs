use migration_connector::DatabaseSchema;
use sql_schema_describer::{walkers::SqlSchemaExt, SqlSchema, Table};

#[derive(Default, Debug)]
pub(crate) struct SqlDatabaseSchema {
    pub(crate) describer_schema: SqlSchema,
    /// A **sorted** array of (table_idx, column_idx) with prisma-level defaults.
    pub(crate) prisma_level_defaults: Vec<(u32, u32)>,
}

impl SqlDatabaseSchema {
    pub(crate) fn from_erased(erased: DatabaseSchema) -> Box<Self> {
        erased.downcast()
    }

    pub(crate) fn tables(&self) -> &[Table] {
        &self.describer_schema.tables
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
    fn enum_walker_at(&self, index: usize) -> sql_schema_describer::walkers::EnumWalker<'_> {
        self.describer_schema.enum_walker_at(index)
    }

    fn table_walker<'a>(&'a self, name: &str) -> Option<sql_schema_describer::walkers::TableWalker<'a>> {
        self.describer_schema.table_walker(name)
    }

    fn table_walker_at(
        &self,
        table_id: sql_schema_describer::TableId,
    ) -> sql_schema_describer::walkers::TableWalker<'_> {
        self.describer_schema.table_walker_at(table_id)
    }

    fn view_walker_at(&self, index: usize) -> sql_schema_describer::walkers::ViewWalker<'_> {
        self.describer_schema.view_walker_at(index)
    }

    fn udt_walker_at(&self, index: usize) -> sql_schema_describer::walkers::UserDefinedTypeWalker<'_> {
        self.describer_schema.udt_walker_at(index)
    }
}
