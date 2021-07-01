//! Functions and types for conveniently traversing and querying a SqlSchema.

#![deny(missing_docs)]

use crate::{
    Column, ColumnArity, ColumnId, ColumnType, ColumnTypeFamily, DefaultValue, Enum, ForeignKey, ForeignKeyAction,
    Index, IndexType, PrimaryKey, SqlSchema, Table, TableId, UserDefinedType, View,
};
use serde::de::DeserializeOwned;
use std::fmt;

/// Traverse all the columns in the schema.
pub fn walk_columns(schema: &SqlSchema) -> impl Iterator<Item = ColumnWalker<'_>> {
    schema.iter_tables().flat_map(move |(table_id, table)| {
        (0..table.columns.len()).map(move |column_id| ColumnWalker {
            schema,
            column_id: ColumnId(column_id as u32),
            table_id,
        })
    })
}

/// Traverse a table column.
#[derive(Clone, Copy)]
pub struct ColumnWalker<'a> {
    /// The schema the column is contained in.
    schema: &'a SqlSchema,
    column_id: ColumnId,
    table_id: TableId,
}

impl<'a> fmt::Debug for ColumnWalker<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColumnWalker")
            .field("column_id", &self.column_id)
            .field("table_id", &self.table_id)
            .finish()
    }
}

impl<'a> ColumnWalker<'a> {
    /// The nullability and arity of the column.
    pub fn arity(&self) -> &ColumnArity {
        &self.column().tpe.arity
    }

    /// A reference to the underlying Column struct.
    pub fn column(&self) -> &'a Column {
        &self.table().table()[self.column_id]
    }

    /// The index of the column in the parent table.
    pub fn column_id(&self) -> ColumnId {
        self.column_id
    }

    /// Returns whether the column has the enum default value of the given enum type.
    pub fn column_has_enum_default_value(&self, enum_name: &str, value: &str) -> bool {
        self.column_type_family_as_enum().map(|enm| enm.name.as_str()) == Some(enum_name)
            && self
                .default()
                .and_then(|default| default.as_value())
                .and_then(|value| value.as_enum_value())
                == Some(value)
    }

    /// Returns whether the type of the column matches the provided enum name.
    pub fn column_type_is_enum(&self, enum_name: &str) -> bool {
        self.column_type_family_as_enum()
            .map(|enm| enm.name == enum_name)
            .unwrap_or(false)
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

    /// The column native type.
    pub fn column_native_type<T>(&self) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.column()
            .tpe
            .native_type
            .as_ref()
            .map(|val| serde_json::from_value(val.clone()).unwrap())
    }

    /// Is this column an auto-incrementing integer?
    pub fn is_autoincrement(&self) -> bool {
        self.column().auto_increment
    }

    /// Is this column a part of the table's primary key?
    pub fn is_part_of_primary_key(&self) -> bool {
        self.table().table().is_part_of_primary_key(self.name())
    }

    /// Is this column a part of the table's primary key?
    pub fn is_part_of_foreign_key(&self) -> bool {
        self.table().table().is_part_of_foreign_key(self.name())
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
            table_id: self.table_id,
        }
    }

    /// Get a reference to the SQL schema the column is part of.
    pub fn schema(&self) -> &'a SqlSchema {
        self.schema
    }
}

/// Traverse a view
#[derive(Clone, Copy)]
pub struct ViewWalker<'a> {
    /// The schema the view is contained in.
    schema: &'a SqlSchema,
    /// The index of the view in the schema.
    view_index: usize,
}

impl<'a> ViewWalker<'a> {
    /// Create a ViewWalker from a schema and a reference to one of its views.
    pub fn new(schema: &'a SqlSchema, view_index: usize) -> Self {
        Self { schema, view_index }
    }

    /// The name of the view
    pub fn name(&self) -> &'a str {
        &self.view().name
    }

    /// The SQL definition of the view
    pub fn definition(&self) -> Option<&'a str> {
        self.view().definition.as_deref()
    }

    /// The index of the view in the schema.
    pub fn view_index(&self) -> usize {
        self.view_index
    }

    fn view(&self) -> &'a View {
        &self.schema.views[self.view_index]
    }
}

/// Traverse a user-defined type
#[derive(Clone, Copy)]
pub struct UserDefinedTypeWalker<'a> {
    schema: &'a SqlSchema,
    udt_index: usize,
}

impl<'a> UserDefinedTypeWalker<'a> {
    /// Create a UserDefinedTypeWalker from a schema and a reference to one of its udts.
    pub fn new(schema: &'a SqlSchema, udt_index: usize) -> Self {
        Self { schema, udt_index }
    }

    /// The name of the type
    pub fn name(&self) -> &'a str {
        &self.udt().name
    }

    /// The SQL definition of the type
    pub fn definition(&self) -> Option<&'a str> {
        self.udt().definition.as_deref()
    }

    /// The index of the user-defined type in the schema.
    pub fn udt_index(&self) -> usize {
        self.udt_index
    }

    fn udt(&self) -> &'a UserDefinedType {
        &self.schema.user_defined_types[self.udt_index]
    }
}

/// Traverse a table.
#[derive(Clone, Copy)]
pub struct TableWalker<'a> {
    /// The schema the table is contained in.
    schema: &'a SqlSchema,
    table_id: TableId,
}

impl<'a> fmt::Debug for TableWalker<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableWalker").field("table_id", &self.table_id).finish()
    }
}

impl<'a> TableWalker<'a> {
    /// Create a TableWalker from a schema and a reference to one of its tables. This should stay private.
    pub(crate) fn new(schema: &'a SqlSchema, table_id: TableId) -> Self {
        Self { schema, table_id }
    }

    /// Get a column in the table, by name.
    pub fn column(&self, column_name: &str) -> Option<ColumnWalker<'a>> {
        self.columns().find(|column| column.name() == column_name)
    }

    /// Get a column in the table by index.
    pub fn column_at(&self, column_id: ColumnId) -> ColumnWalker<'a> {
        ColumnWalker {
            schema: self.schema,
            column_id,
            table_id: self.table_id,
        }
    }

    /// Traverse the table's columns.
    pub fn columns(&self) -> impl Iterator<Item = ColumnWalker<'a>> {
        let schema = self.schema;
        let table_id = self.table_id;

        (0..self.table().columns.len()).map(move |column_id| ColumnWalker {
            schema,
            column_id: ColumnId(column_id as u32),
            table_id,
        })
    }

    /// The number of foreign key constraints on the table.
    pub fn foreign_key_count(&self) -> usize {
        self.table().foreign_keys.len()
    }

    /// Traverse to an index by index.
    pub fn index_at(&self, index_index: usize) -> IndexWalker<'a> {
        IndexWalker {
            schema: self.schema,
            table_id: self.table_id,
            index_index,
        }
    }

    /// Traverse the indexes on the table.
    pub fn indexes(&self) -> impl Iterator<Item = IndexWalker<'a>> {
        let schema = self.schema;
        let table_id = self.table_id;

        (0..self.table().indices.len()).map(move |index_index| IndexWalker {
            schema,
            table_id,
            index_index,
        })
    }

    /// The number of indexes on the table.
    pub fn indexes_count(&self) -> usize {
        self.table().indices.len()
    }

    /// Traverse the foreign keys on the table.
    pub fn foreign_keys(&self) -> impl Iterator<Item = ForeignKeyWalker<'a>> {
        let table_id = self.table_id;
        let schema = self.schema;

        (0..self.table().foreign_keys.len()).map(move |foreign_key_index| ForeignKeyWalker {
            foreign_key_index,
            table_id,
            schema,
        })
    }

    /// Traverse foreign keys from other tables, referencing current table.
    pub fn referencing_foreign_keys(&self) -> impl Iterator<Item = ForeignKeyWalker<'a>> {
        let table_id = self.table_id;

        self.schema
            .table_walkers()
            .filter(move |t| t.table_id() != table_id)
            .flat_map(|t| t.foreign_keys())
            .filter(move |fk| fk.referenced_table().table_id() == table_id)
    }

    /// Get a foreign key by index.
    pub fn foreign_key_at(&self, index: usize) -> ForeignKeyWalker<'a> {
        ForeignKeyWalker {
            schema: self.schema,
            table_id: self.table_id,
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
        &self.schema[self.table_id]
    }

    /// The index of the table in the schema.
    pub fn table_id(&self) -> TableId {
        self.table_id
    }
}

/// Traverse a foreign key.
#[derive(Clone, Copy)]
pub struct ForeignKeyWalker<'schema> {
    /// The index of the foreign key in the table.
    foreign_key_index: usize,
    table_id: TableId,
    schema: &'schema SqlSchema,
}

impl<'a> fmt::Debug for ForeignKeyWalker<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForeignKeyWalker")
            .field("foreign_key_index", &self.foreign_key_index)
            .field("table_id", &self.table_id)
            .field("__table_name", &self.table().name())
            .field("__referenced_table", &self.foreign_key().referenced_table)
            .field("__constrained_columns", &self.foreign_key().columns)
            .field("__referenced_columns", &self.foreign_key().referenced_columns)
            .finish()
    }
}

impl<'schema> ForeignKeyWalker<'schema> {
    /// The names of the foreign key columns on the referencing table.
    pub fn constrained_column_names(&self) -> &'schema [String] {
        &self.foreign_key().columns
    }

    /// The foreign key columns on the referencing table.
    pub fn constrained_columns<'b>(&'b self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'b {
        self.foreign_key()
            .columns
            .iter()
            .filter_map(move |colname| self.table().columns().find(|column| colname == column.name()))
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
    pub fn referenced_column_names(&self) -> &'schema [String] {
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
            table_id: self
                .schema
                .table_walker(&self.foreign_key().referenced_table)
                .ok_or_else(|| format!("Foreign key references unknown table. {:?}", self))
                .unwrap()
                .table_id,
        }
    }

    /// Traverse to the referencing/constrained table.
    pub fn table(&self) -> TableWalker<'schema> {
        TableWalker {
            schema: self.schema,
            table_id: self.table_id,
        }
    }

    /// True if relation is back to the same table.
    pub fn is_self_relation(&self) -> bool {
        self.table().name() == self.referenced_table().name()
    }
}

/// Traverse an index.
#[derive(Clone, Copy)]
pub struct IndexWalker<'a> {
    schema: &'a SqlSchema,
    /// The index of the table in the schema.
    table_id: TableId,
    /// The index of the database index in the table.
    index_index: usize,
}

impl<'a> fmt::Debug for IndexWalker<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IndexWalker")
            .field("index_index", &self.index_index)
            .field("table_id", &self.table_id)
            .finish()
    }
}

impl<'a> IndexWalker<'a> {
    /// The names of the indexed columns.
    pub fn column_names(&self) -> &'a [String] {
        &self.get().columns
    }

    /// Traverse the indexed columns.
    pub fn columns<'b>(&'b self) -> impl Iterator<Item = ColumnWalker<'a>> + 'b {
        self.get().columns.iter().map(move |column_name| {
            self.table()
                .column(column_name)
                .expect("Failed to find column referenced in index")
        })
    }

    /// True if index contains the given column.
    pub fn contains_column(&self, column_name: &str) -> bool {
        self.get().columns.iter().any(|column| column == column_name)
    }

    fn get(&self) -> &'a Index {
        &self.table().table().indices[self.index_index]
    }

    /// The index of the index in the parent table.
    pub fn index(&self) -> usize {
        self.index_index
    }

    /// The IndexType
    pub fn index_type(&self) -> &'a IndexType {
        &self.get().tpe
    }

    /// The name of the index.
    pub fn name(&self) -> &'a str {
        &self.get().name
    }

    /// Traverse to the table of the index.
    pub fn table(&self) -> TableWalker<'a> {
        TableWalker {
            table_id: self.table_id,
            schema: self.schema,
        }
    }
}

/// Traverse an enum.
#[derive(Clone, Copy)]
pub struct EnumWalker<'a> {
    pub(crate) schema: &'a SqlSchema,
    pub(crate) enum_index: usize,
}

impl<'a> fmt::Debug for EnumWalker<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnumWalker")
            .field("enum_index", &self.enum_index)
            .finish()
    }
}

impl<'a> EnumWalker<'a> {
    /// The index of the enum in the parent schema.
    pub fn enum_index(&self) -> usize {
        self.enum_index
    }

    fn get(&self) -> &'a Enum {
        &self.schema.enums[self.enum_index]
    }

    /// The name of the enum. This is a made up name on MySQL.
    pub fn name(&self) -> &'a str {
        &self.get().name
    }

    /// The values of the enum
    pub fn values(&self) -> &'a [String] {
        &self.get().values
    }
}

/// Extension methods for the traversal of a SqlSchema.
pub trait SqlSchemaExt {
    /// Find an enum by index.
    fn enum_walker_at(&self, index: usize) -> EnumWalker<'_>;

    /// Find a table by name.
    fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>>;

    /// Find a table by id.
    fn table_walker_at(&self, table_id: TableId) -> TableWalker<'_>;

    /// Find a view by index.
    fn view_walker_at(&self, index: usize) -> ViewWalker<'_>;

    /// Find a user-defined type by index.
    fn udt_walker_at(&self, index: usize) -> UserDefinedTypeWalker<'_>;
}

impl SqlSchemaExt for SqlSchema {
    fn enum_walker_at(&self, index: usize) -> EnumWalker<'_> {
        EnumWalker {
            schema: self,
            enum_index: index,
        }
    }

    fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>> {
        Some(TableWalker {
            table_id: TableId(self.tables.iter().position(|table| table.name == name)? as u32),
            schema: self,
        })
    }

    fn table_walker_at(&self, table_id: TableId) -> TableWalker<'_> {
        TableWalker { table_id, schema: self }
    }

    fn view_walker_at(&self, index: usize) -> ViewWalker<'_> {
        ViewWalker {
            view_index: index,
            schema: self,
        }
    }

    fn udt_walker_at(&self, index: usize) -> UserDefinedTypeWalker<'_> {
        UserDefinedTypeWalker {
            udt_index: index,
            schema: self,
        }
    }
}
