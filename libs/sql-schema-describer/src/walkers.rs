//! Functions and types for conveniently traversing and querying a SqlSchema.

#![deny(missing_docs)]

use crate::{
    Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue, Enum, ForeignKey, ForeignKeyAction, Index,
    IndexType, PrimaryKey, SqlSchema, Table,
};

/// Traverse all the columns in the schema.
pub fn walk_columns<'a>(schema: &'a SqlSchema) -> impl Iterator<Item = ColumnWalker<'a>> + 'a {
    schema.tables.iter().enumerate().flat_map(move |(table_index, table)| {
        (0..table.columns.len()).map(move |column_index| ColumnWalker {
            schema,
            column_index,
            table_index,
        })
    })
}

/// Traverse a table column.
#[derive(Debug, Clone, Copy)]
pub struct ColumnWalker<'a> {
    /// The schema the column is contained in.
    schema: &'a SqlSchema,
    /// The index of the column in the table.
    column_index: usize,
    /// The index of the table in the schema.
    table_index: usize,
}

impl<'a> ColumnWalker<'a> {
    /// The nullability and arity of the column.
    pub fn arity(&self) -> &ColumnArity {
        &self.column().tpe.arity
    }

    /// A reference to the underlying Column struct.
    pub fn column(&self) -> &'a Column {
        &self.table().table().columns[self.column_index]
    }

    /// The index of the column in the parent table.
    pub fn column_index(&self) -> usize {
        self.column_index
    }

    /// The type family.
    pub fn column_type_family(&self) -> &'a ColumnTypeFamily {
        &self.column().tpe.family
    }

    /// Extract an `Enum` column type family, or `None` if the family is something else.
    pub fn column_type_family_as_enum(&self) -> Option<&'a Enum> {
        self.column_type_family().as_enum().map(|enum_name| {
            self.schema()
                .get_enum(enum_name)
                .ok_or_else(|| panic!("Cannot find enum referenced in ColumnTypeFamily (`{}`)", enum_name))
                .unwrap()
        })
    }

    /// The column name.
    pub fn name(&self) -> &'a str {
        &self.column().name
    }

    /// The default value for the column.
    pub fn default(&self) -> Option<&'a DefaultValue> {
        self.column().default.as_ref()
    }

    /// The full column type.
    pub fn column_type(&self) -> &'a ColumnType {
        &self.column().tpe
    }

    /// Is this column an auto-incrementing integer?
    pub fn is_autoincrement(&self) -> bool {
        self.column().auto_increment
    }

    /// Returns whether two columns are named the same and belong to the same table.
    pub fn is_same_column(&self, other: &ColumnWalker<'_>) -> bool {
        self.name() == other.name() && self.table().name() == other.table().name()
    }

    /// Returns whether this column is the primary key. If it is only part of the primary key, this will return false.
    pub fn is_single_primary_key(&self) -> bool {
        self.table()
            .primary_key()
            .map(|pk| pk.columns == [self.name()])
            .unwrap_or(false)
    }

    /// Traverse to the column's table.
    pub fn table(&self) -> TableWalker<'a> {
        TableWalker {
            schema: self.schema,
            table_index: self.table_index,
        }
    }

    /// Get a reference to the SQL schema the column is part of.
    pub fn schema(&self) -> &'a SqlSchema {
        self.schema
    }
}

/// Traverse a table.
#[derive(Clone, Copy)]
pub struct TableWalker<'a> {
    /// The schema the column is contained in.
    schema: &'a SqlSchema,
    /// The index of the table in the schema.
    table_index: usize,
}

impl<'a> TableWalker<'a> {
    /// Create a TableWalker from a schema and a reference to one of its tables. This should stay private.
    pub(crate) fn new(schema: &'a SqlSchema, table_index: usize) -> Self {
        Self { schema, table_index }
    }

    /// Get a column in the table, by name.
    pub fn column(&self, column_name: &str) -> Option<ColumnWalker<'a>> {
        self.columns().find(|column| column.name() == column_name)
    }

    /// Get a column in the table by index.
    pub fn column_at(&self, idx: usize) -> ColumnWalker<'a> {
        ColumnWalker {
            schema: self.schema,
            column_index: idx,
            table_index: self.table_index,
        }
    }

    /// Traverse the table's columns.
    pub fn columns(&self) -> impl Iterator<Item = ColumnWalker<'a>> {
        let schema = self.schema;
        let table_index = self.table_index;

        (0..self.table().columns.len()).map(move |column_index| ColumnWalker {
            schema,
            column_index,
            table_index,
        })
    }

    /// The number of foreign key constraints on the table.
    pub fn foreign_key_count(&self) -> usize {
        self.table().foreign_keys.len()
    }

    /// Traverse the indexes on the table.
    pub fn indexes(&self) -> impl Iterator<Item = IndexWalker<'a>> {
        let schema = self.schema;
        let table_index = self.table_index;

        (0..self.table().indices.len()).map(move |index_index| IndexWalker {
            schema,
            table_index,
            index_index,
        })
    }

    /// Traverse the foreign keys on the table.
    pub fn foreign_keys(&self) -> impl Iterator<Item = ForeignKeyWalker<'a>> {
        let table_index = self.table_index;
        let schema = self.schema;

        (0..self.table().foreign_keys.len()).map(move |foreign_key_index| ForeignKeyWalker {
            foreign_key_index,
            table_index,
            schema,
        })
    }

    /// Get a foreign key by index.
    pub fn foreign_key_at(&self, index: usize) -> ForeignKeyWalker<'a> {
        ForeignKeyWalker {
            schema: self.schema,
            table_index: self.table_index,
            foreign_key_index: index,
        }
    }

    /// The table name.
    pub fn name(&self) -> &'a str {
        &self.table().name
    }

    /// Try to traverse a foreign key for a single column.
    pub fn foreign_key_for_column(&self, column: &str) -> Option<&'a ForeignKey> {
        self.table().foreign_key_for_column(column)
    }

    /// Traverse to the primary key of the table.
    pub fn primary_key(&self) -> Option<&'a PrimaryKey> {
        self.table().primary_key.as_ref()
    }

    /// The names of the columns that are part of the primary key. `None` means
    /// there is no primary key on the table.
    pub fn primary_key_column_names(&self) -> Option<&[String]> {
        self.table().primary_key.as_ref().map(|pk| pk.columns.as_slice())
    }

    /// Reference to the underlying `Table` struct.
    pub fn table(&self) -> &'a Table {
        &self.schema.tables[self.table_index]
    }

    /// The index of the table in the schema.
    pub fn table_index(&self) -> usize {
        self.table_index
    }
}

/// Traverse a foreign key.
pub struct ForeignKeyWalker<'schema> {
    /// The index of the foreign key in the table.
    foreign_key_index: usize,
    /// The index of the table in the schema.
    table_index: usize,
    schema: &'schema SqlSchema,
}

impl<'schema> ForeignKeyWalker<'schema> {
    /// The names of the foreign key columns on the referencing table.
    pub fn constrained_column_names(&self) -> &[String] {
        &self.foreign_key().columns
    }

    /// The foreign key columns on the referencing table.
    pub fn constrained_columns<'b>(&'b self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'b {
        self.table().columns().filter(move |column| {
            self.foreign_key()
                .columns
                .iter()
                .any(|colname| colname == column.name())
        })
    }

    /// The name of the foreign key constraint.
    pub fn constraint_name(&self) -> Option<&'schema str> {
        self.foreign_key().constraint_name.as_deref()
    }

    /// The underlying ForeignKey struct.
    pub fn foreign_key(&self) -> &'schema ForeignKey {
        &self.table().table().foreign_keys[self.foreign_key_index]
    }

    /// The index of the foreign key in the parent table.
    pub fn foreign_key_index(&self) -> usize {
        self.foreign_key_index
    }

    /// Access the underlying ForeignKey struct.
    pub fn inner(&self) -> &'schema ForeignKey {
        self.foreign_key()
    }

    /// The `ON DELETE` behaviour of the foreign key.
    pub fn on_delete_action(&self) -> &ForeignKeyAction {
        &self.foreign_key().on_delete_action
    }

    /// The `ON UPDATE` behaviour of the foreign key.
    pub fn on_update_action(&self) -> &ForeignKeyAction {
        &self.foreign_key().on_update_action
    }

    /// The names of the columns referenced by the foreign key on the referenced table.
    pub fn referenced_column_names(&self) -> &[String] {
        &self.foreign_key().referenced_columns
    }

    /// The number of columns referenced by the constraint.
    pub fn referenced_columns_count(&self) -> usize {
        self.foreign_key().referenced_columns.len()
    }

    /// The table the foreign key "points to".
    pub fn referenced_table(&self) -> TableWalker<'schema> {
        TableWalker {
            schema: self.schema,
            table_index: self
                .schema
                .table_walker(&self.foreign_key().referenced_table)
                .expect("foreign key references unknown table")
                .table_index,
        }
    }

    /// Traverse to the referencing/constrained table.
    pub fn table(&self) -> TableWalker<'schema> {
        TableWalker {
            schema: self.schema,
            table_index: self.table_index,
        }
    }
}

/// Traverse an index.
pub struct IndexWalker<'a> {
    schema: &'a SqlSchema,
    /// The index of the table in the schema.
    table_index: usize,
    /// The index of the database index in the table.
    index_index: usize,
}

impl<'a> IndexWalker<'a> {
    /// The names of the indexed columns.
    pub fn column_names(&self) -> &[String] {
        &self.index().columns
    }

    /// Traverse the indexed columns.
    pub fn columns<'b>(&'b self) -> impl Iterator<Item = ColumnWalker<'a>> + 'b {
        self.index().columns.iter().map(move |column_name| {
            self.table()
                .column(column_name)
                .expect("Failed to find column referenced in index")
        })
    }

    /// The underlying index struct.
    pub fn index(&self) -> &'a Index {
        &self.table().table().indices[self.index_index]
    }

    /// The IndexType
    pub fn index_type(&self) -> &IndexType {
        &self.index().tpe
    }

    /// The name of the index.
    pub fn name(&self) -> &str {
        &self.index().name
    }

    /// Traverse to the table of the index.
    pub fn table(&self) -> TableWalker<'a> {
        TableWalker {
            table_index: self.table_index,
            schema: self.schema,
        }
    }
}

/// Extension methods for the traversal of a SqlSchema.
pub trait SqlSchemaExt {
    /// Find a table by name.
    fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>>;

    /// Find a table by index.
    fn table_walker_at(&self, index: usize) -> TableWalker<'_>;
}

impl SqlSchemaExt for SqlSchema {
    fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>> {
        Some(TableWalker {
            table_index: self.tables.iter().position(|table| table.name == name)?,
            schema: self,
        })
    }

    fn table_walker_at(&self, index: usize) -> TableWalker<'_> {
        TableWalker {
            table_index: index,
            schema: self,
        }
    }
}
