use crate::{
    Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue, Enum, ForeignKey, PrimaryKey, SqlSchema, Table,
};

pub fn walk_columns<'a>(schema: &'a SqlSchema) -> impl Iterator<Item = ColumnWalker<'a>> + 'a {
    schema.tables.iter().flat_map(move |table| {
        table
            .columns
            .iter()
            .map(move |column| ColumnWalker { schema, column, table })
    })
}

pub fn find_column<'a>(schema: &'a SqlSchema, table_name: &str, column_name: &str) -> Option<ColumnWalker<'a>> {
    schema
        .tables
        .iter()
        .find(move |table| table.name == table_name)
        .and_then(move |table| {
            table
                .columns
                .iter()
                .find(|column| column.name == column_name)
                .map(|column| ColumnWalker { schema, table, column })
        })
}

#[derive(Debug, Clone, Copy)]
pub struct ColumnWalker<'a> {
    pub schema: &'a SqlSchema,
    pub column: &'a Column,
    pub table: &'a Table,
}

impl<'a> ColumnWalker<'a> {
    pub fn arity(&self) -> &ColumnArity {
        &self.column.tpe.arity
    }

    pub fn column_type_family(&self) -> &'a ColumnTypeFamily {
        &self.column.tpe.family
    }

    pub fn column_type_family_as_enum(&self) -> Option<&'a Enum> {
        self.column_type_family().as_enum().map(|enum_name| {
            self.schema()
                .get_enum(enum_name)
                .ok_or_else(|| panic!("Cannot find enum referenced in ColumnTypeFamily (`{}`)", enum_name))
                .unwrap()
        })
    }

    pub fn name(&self) -> &'a str {
        &self.column.name
    }

    pub fn default(&self) -> Option<&'a DefaultValue> {
        self.column.default.as_ref()
    }

    pub fn column_type(&self) -> &'a ColumnType {
        &self.column.tpe
    }

    pub fn is_autoincrement(&self) -> bool {
        self.column.auto_increment
    }

    pub fn is_same_column(&self, other: &ColumnWalker<'_>) -> bool {
        self.name() == other.name() && self.table().name() == other.table().name()
    }

    /// Returns whether this column is the primary key. If it is only part of the primary key, this will return false.
    pub fn is_single_primary_key(&self) -> bool {
        self.table()
            .primary_key()
            .map(|pk| pk.columns == &[self.name()])
            .unwrap_or(false)
    }

    pub fn table(&self) -> TableWalker<'a> {
        TableWalker {
            schema: self.schema,
            table: self.table,
        }
    }

    pub fn schema(&self) -> &'a SqlSchema {
        self.schema
    }
}

#[derive(Clone, Copy)]
pub struct TableWalker<'a> {
    pub schema: &'a SqlSchema,
    pub table: &'a Table,
}

impl<'a> TableWalker<'a> {
    pub fn new(schema: &'a SqlSchema, table: &'a Table) -> Self {
        Self { schema, table }
    }

    pub fn column(&self, column_name: &str) -> Option<ColumnWalker<'a>> {
        self.columns().find(|column| column.name() == column_name)
    }

    pub fn columns<'b>(&'b self) -> impl Iterator<Item = ColumnWalker<'a>> + 'b {
        self.table.columns.iter().map(move |column| ColumnWalker {
            column,
            schema: self.schema,
            table: self.table,
        })
    }

    pub fn foreign_keys<'b>(&'b self) -> impl Iterator<Item = ForeignKeyWalker<'b, 'a>> + 'b {
        self.table.foreign_keys.iter().map(move |foreign_key| ForeignKeyWalker {
            foreign_key,
            table: self,
        })
    }

    pub fn name(&self) -> &'a str {
        &self.table.name
    }

    pub fn foreign_key_for_column(&self, column: &str) -> Option<&'a ForeignKey> {
        self.table.foreign_key_for_column(column)
    }

    pub fn primary_key(&self) -> Option<&'a PrimaryKey> {
        self.table.primary_key.as_ref()
    }
}

pub struct ForeignKeyWalker<'a, 'schema> {
    table: &'a TableWalker<'schema>,
    foreign_key: &'schema ForeignKey,
}

impl<'a, 'schema> ForeignKeyWalker<'a, 'schema> {
    pub fn constrained_columns<'b>(&'b self) -> impl Iterator<Item = ColumnWalker<'a>> + 'b {
        self.table()
            .columns()
            .filter(move |column| self.foreign_key.columns.contains(&column.column.name))
    }

    pub fn constraint_name(&self) -> Option<&'a str> {
        self.foreign_key.constraint_name.as_deref()
    }

    pub fn inner(&self) -> &'a ForeignKey {
        self.foreign_key
    }

    pub fn referenced_columns_count(&self) -> usize {
        self.foreign_key.referenced_columns.len()
    }

    pub fn referenced_table(&self) -> TableWalker<'a> {
        TableWalker {
            schema: self.table.schema,
            table: self
                .table
                .schema
                .table(&self.foreign_key.referenced_table)
                .expect("foreign key references unknown table"),
        }
    }

    pub fn table(&self) -> &'a TableWalker<'schema> {
        self.table
    }
}

pub trait SqlSchemaExt {
    fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>>;
}

impl SqlSchemaExt for SqlSchema {
    fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>> {
        Some(TableWalker {
            table: self.table(name).ok()?,
            schema: self,
        })
    }
}
