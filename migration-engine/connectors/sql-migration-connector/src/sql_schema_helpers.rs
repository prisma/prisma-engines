use sql_schema_describer::{Column, ColumnType, ForeignKey, SqlSchema, Table};

pub(crate) fn walk_columns<'a>(schema: &'a SqlSchema) -> impl Iterator<Item = ColumnRef<'a>> + 'a {
    schema.tables.iter().flat_map(move |table| {
        table
            .columns
            .iter()
            .map(move |column| ColumnRef { schema, column, table })
    })
}

pub(crate) struct ColumnRef<'a> {
    pub(crate) schema: &'a SqlSchema,
    pub(crate) column: &'a Column,
    pub(crate) table: &'a Table,
}

impl<'a> ColumnRef<'a> {
    pub(crate) fn name(&self) -> &'a str {
        &self.column.name
    }

    pub(crate) fn default(&self) -> Option<&'a str> {
        self.column.default.as_ref().map(String::as_str)
    }

    pub(crate) fn column_type(&self) -> &'a ColumnType {
        &self.column.tpe
    }

    pub(crate) fn auto_increment(&self) -> bool {
        self.column.auto_increment
    }

    pub(crate) fn is_required(&self) -> bool {
        self.column.is_required()
    }

    pub(crate) fn table(&self) -> TableRef<'a> {
        TableRef {
            _schema: self.schema,
            table: self.table,
        }
    }

    pub(crate) fn schema(&self) -> &'a SqlSchema {
        self.schema
    }
}

pub(crate) struct TableRef<'a> {
    _schema: &'a SqlSchema,
    table: &'a Table,
}

impl<'a> TableRef<'a> {
    pub(crate) fn name(&self) -> &'a str {
        &self.table.name
    }

    pub(crate) fn foreign_key_for_column(&self, column: &str) -> Option<&'a ForeignKey> {
        self.table.foreign_key_for_column(column)
    }
}
