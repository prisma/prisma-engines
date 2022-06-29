//! Functions and types for conveniently traversing and querying a SqlSchema.

#![deny(missing_docs)]

use crate::{
    ids::*, Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue, Enum, ForeignKey, ForeignKeyAction, Index,
    IndexColumn, IndexType, PrimaryKey, PrimaryKeyColumn, SQLSortOrder, SqlSchema, Table, UserDefinedType, View,
};
use serde::de::DeserializeOwned;
use std::ops::Range;

/// Traverse all the columns in the schema.
pub fn walk_columns(schema: &SqlSchema) -> impl Iterator<Item = ColumnWalker<'_>> {
    (0..schema.columns.len()).map(|idx| ColumnWalker {
        schema,
        id: ColumnId(idx as u32),
    })
}

/// A generic reference to a schema item. It holds a reference to the schema so it can offer a
/// convenient API based on the Id type.
#[derive(Clone, Copy, Debug)]
pub struct Walker<'a, Id> {
    /// The identifier.
    pub id: Id,
    /// The schema for which the identifier is valid.
    pub schema: &'a SqlSchema,
}

impl<'a, Id> Walker<'a, Id> {
    /// Make another walker for the same schema.
    pub fn jump<T>(self, other_id: T) -> Walker<'a, T> {
        Walker {
            schema: self.schema,
            id: other_id,
        }
    }
}

/// Traverse a foreign key.
pub type ForeignKeyWalker<'a> = Walker<'a, ForeignKeyId>;

/// Traverse column.
pub type ColumnWalker<'a> = Walker<'a, ColumnId>;

/// Traverse a table.
pub type TableWalker<'a> = Walker<'a, TableId>;

/// Traverse an enum.
pub type EnumWalker<'a> = Walker<'a, EnumId>;

/// Traverse an index.
pub type IndexWalker<'a> = Walker<'a, IndexId>;

impl<'a> ColumnWalker<'a> {
    /// The nullability and arity of the column.
    pub fn arity(&self) -> &ColumnArity {
        &self.column().tpe.arity
    }

    /// A reference to the underlying Column struct.
    pub fn column(&self) -> &'a Column {
        &self.schema[self.id].1
    }

    fn table_id(&self) -> TableId {
        self.schema[self.id].0
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
            self.schema
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
    pub fn column_type(self) -> &'a ColumnType {
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

    /// Is this column indexed by a secondary index??
    pub fn is_part_of_secondary_index(&self) -> bool {
        let table = self.table();
        let name = self.name();
        table.indexes().any(|idx| idx.contains_column(name))
    }

    /// Is this column a part of the table's primary key?
    pub fn is_part_of_primary_key(&self) -> bool {
        match self.table().primary_key() {
            Some(pk) => pk.columns.iter().any(|c| c.name() == self.name()),
            None => false,
        }
    }

    /// Is this column a part of one of the table's foreign keys?
    pub fn is_part_of_foreign_key(self) -> bool {
        let column_id = self.id;
        self.table()
            .foreign_keys()
            .any(|fk| fk.constrained_columns().any(|col| col.id == column_id))
    }

    /// Returns whether two columns are named the same and belong to the same table.
    pub fn is_same_column(&self, other: ColumnWalker<'_>) -> bool {
        self.name() == other.name() && self.table().name() == other.table().name()
    }

    /// Returns whether this column is the primary key. If it is only part of the primary key, this will return false.
    pub fn is_single_primary_key(&self) -> bool {
        self.table()
            .primary_key()
            .map(|pk| pk.columns.len() == 1 && pk.columns.first().map(|c| c.name() == self.name()).unwrap_or(false))
            .unwrap_or(false)
    }

    /// Traverse to the column's table.
    pub fn table(&self) -> TableWalker<'a> {
        TableWalker {
            schema: self.schema,
            id: self.table_id(),
        }
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

impl<'a> TableWalker<'a> {
    /// Get a column in the table, by name.
    pub fn column(&self, column_name: &str) -> Option<ColumnWalker<'a>> {
        self.columns().find(|column| column.name() == column_name)
    }

    /// Get a column in the table, by name.
    pub fn column_case_insensitive(&self, column_name: &str) -> Option<ColumnWalker<'a>> {
        self.columns().find(|column| column.name() == column_name)
    }

    fn columns_range(self) -> Range<usize> {
        range_for_key(&self.schema.columns, self.id, |(tid, _)| *tid)
    }

    /// Traverse the table's columns.
    pub fn columns(self) -> impl ExactSizeIterator<Item = ColumnWalker<'a>> {
        self.columns_range()
            .into_iter()
            .map(move |idx| self.jump(ColumnId(idx as u32)))
    }

    /// The number of foreign key constraints on the table.
    pub fn foreign_key_count(&self) -> usize {
        self.foreign_keys_range().into_iter().len()
    }

    /// Traverse to an index by id.
    pub fn index_at(&self, index_id: IndexId) -> IndexWalker<'a> {
        self.jump(index_id)
    }

    /// Traverse the indexes on the table.
    pub fn indexes(self) -> impl ExactSizeIterator<Item = IndexWalker<'a>> {
        let table_id = self.id;

        (0..self.table().indices.len()).map(move |index_index| self.jump(IndexId(table_id, index_index as u32)))
    }

    /// Traverse the foreign keys on the table.
    pub fn foreign_keys(self) -> impl Iterator<Item = ForeignKeyWalker<'a>> {
        self.foreign_keys_range()
            .map(move |id| self.jump(ForeignKeyId(id as u32)))
    }

    /// Traverse foreign keys from other tables, referencing current table.
    pub fn referencing_foreign_keys(self) -> impl Iterator<Item = ForeignKeyWalker<'a>> {
        let table_id = self.id;
        self.schema
            .table_walkers()
            .filter(move |t| t.id != table_id)
            .flat_map(|t| t.foreign_keys())
            .filter(move |fk| fk.referenced_table().id == table_id)
    }

    /// The table name.
    pub fn name(self) -> &'a str {
        &self.table().name
    }

    fn foreign_keys_range(self) -> Range<usize> {
        range_for_key(&self.schema.foreign_keys, self.id, |(id, _)| *id)
    }

    /// Try to traverse a foreign key for a single column.
    pub fn foreign_key_for_column(self, column: &str) -> Option<ForeignKeyWalker<'a>> {
        self.foreign_keys().find(|fk| fk.constrained_column_names() == [column])
    }

    /// Traverse to the primary key of the table.
    pub fn primary_key(self) -> Option<&'a PrimaryKey> {
        self.table().primary_key.as_ref()
    }

    /// The columns that are part of the primary keys.
    pub fn primary_key_columns(&'a self) -> Box<dyn ExactSizeIterator<Item = PrimaryKeyColumnWalker<'a>> + 'a> {
        let as_walker = move |primary_key_column_id: usize, c: &PrimaryKeyColumn| {
            let column_id = self.column(c.name()).map(|c| c.id).unwrap();

            PrimaryKeyColumnWalker {
                schema: self.schema,
                primary_key_column_id,
                table_id: self.id,
                column_id,
            }
        };

        match self.table().primary_key.as_ref() {
            Some(pk) => Box::new(pk.columns.iter().enumerate().map(move |(i, c)| as_walker(i, c))),
            None => Box::new(std::iter::empty()),
        }
    }

    /// The names of the columns that are part of the primary key.
    pub fn primary_key_column_names(self) -> Option<Vec<String>> {
        self.table()
            .primary_key
            .as_ref()
            .map(|pk| pk.columns.iter().map(|c| c.name().to_string()).collect())
    }

    /// Reference to the underlying `Table` struct.
    pub fn table(self) -> &'a Table {
        &self.schema[self.id]
    }
}

/// A walker of a column in a primary key.
#[derive(Clone, Copy)]
pub struct PrimaryKeyColumnWalker<'a> {
    schema: &'a SqlSchema,
    primary_key_column_id: usize,
    table_id: TableId,
    column_id: ColumnId,
}

impl<'a> PrimaryKeyColumnWalker<'a> {
    /// Conversion to a normal column walker.
    pub fn as_column(self) -> ColumnWalker<'a> {
        ColumnWalker {
            schema: self.schema,
            id: self.column_id,
        }
    }

    /// The length limit of the (text) column. Matters on MySQL only.
    pub fn length(self) -> Option<u32> {
        self.get().length
    }

    /// The BTree ordering. Matters on SQL Server only.
    pub fn sort_order(self) -> Option<SQLSortOrder> {
        self.get().sort_order
    }

    fn table(self) -> TableWalker<'a> {
        TableWalker {
            schema: self.schema,
            id: self.table_id,
        }
    }

    fn get(self) -> &'a PrimaryKeyColumn {
        &self.table().primary_key().unwrap().columns[self.primary_key_column_id]
    }
}

impl<'schema> ForeignKeyWalker<'schema> {
    /// The names of the foreign key columns on the referencing table.
    pub fn constrained_column_names(&self) -> &'schema [String] {
        &self.foreign_key().columns
    }

    /// The foreign key columns on the referencing table.
    pub fn constrained_columns<'b>(&'b self) -> impl ExactSizeIterator<Item = ColumnWalker<'schema>> + 'b {
        self.foreign_key()
            .columns
            .iter()
            .map(move |colname| self.table().columns().find(|column| colname == column.name()).unwrap())
    }

    /// The name of the foreign key constraint.
    pub fn constraint_name(&self) -> Option<&'schema str> {
        self.foreign_key().constraint_name.as_deref()
    }

    /// The underlying ForeignKey struct.
    pub fn foreign_key(&self) -> &'schema ForeignKey {
        &self.schema[self.id].1
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
    pub fn referenced_table_name(self) -> &'schema str {
        self.referenced_table().name()
    }

    /// The table the foreign key "points to".
    pub fn referenced_table(&self) -> TableWalker<'schema> {
        self.jump(self.foreign_key().referenced_table)
    }

    /// Traverse to the referencing/constrained table.
    pub fn table(&self) -> TableWalker<'schema> {
        self.jump(self.schema[self.id].0)
    }

    /// True if relation is back to the same table.
    pub fn is_self_relation(&self) -> bool {
        self.table().name() == self.referenced_table().name()
    }
}

/// Traverse an index column.
#[derive(Clone, Copy)]
pub struct IndexColumnWalker<'a> {
    schema: &'a SqlSchema,
    index_column_id: usize,
    index_id: IndexId,
}

impl<'a> IndexColumnWalker<'a> {
    /// Get the index column data.
    pub fn get(&self) -> &'a IndexColumn {
        &self.index().get().columns[self.index_column_id]
    }

    /// The length limit of the (text) column. Matters on MySQL only.
    pub fn length(self) -> Option<u32> {
        self.get().length
    }

    /// The BTree ordering.
    pub fn sort_order(self) -> Option<SQLSortOrder> {
        self.get().sort_order
    }

    /// The table where the column is located.
    pub fn table(&self) -> TableWalker<'a> {
        TableWalker {
            id: self.index_id.0,
            schema: self.schema,
        }
    }

    /// The index of the column.
    pub fn index(&self) -> IndexWalker<'a> {
        IndexWalker {
            schema: self.schema,
            id: self.index_id,
        }
    }

    /// Convert to a normal column walker, losing the possible index arguments.
    pub fn as_column(&self) -> ColumnWalker<'a> {
        let column = self
            .table()
            .columns()
            .find(|c| c.name() == self.get().name())
            .expect("STATE ERROR BOOP");

        ColumnWalker {
            schema: self.schema,
            id: column.id,
        }
    }

    /// The identifier of the index column.
    pub fn index_field_id(&self) -> IndexFieldId {
        IndexFieldId(self.index().id, self.index_column_id as u32)
    }
}

impl<'a> IndexWalker<'a> {
    /// The names of the indexed columns.
    pub fn column_names(&'a self) -> impl ExactSizeIterator<Item = &'a str> + 'a {
        self.get().columns.iter().map(|c| c.name())
    }

    /// Traverse the indexed columns.
    pub fn columns<'b>(&'b self) -> impl ExactSizeIterator<Item = IndexColumnWalker<'a>> + 'b {
        self.get()
            .columns
            .iter()
            .enumerate()
            .map(move |(index_column_id, _)| IndexColumnWalker {
                schema: self.schema,
                index_column_id,
                index_id: self.id,
            })
    }

    /// True if index contains the given column.
    pub fn contains_column(&self, column_name: &str) -> bool {
        self.get().columns.iter().any(|column| column.name() == column_name)
    }

    fn get(&self) -> &'a Index {
        &self.table().table().indices[self.id.1 as usize]
    }

    /// The IndexType
    pub fn index_type(&self) -> IndexType {
        self.get().tpe
    }

    /// The name of the index.
    pub fn name(&self) -> &'a str {
        &self.get().name
    }

    /// Traverse to the table of the index.
    pub fn table(self) -> TableWalker<'a> {
        self.jump(self.id.0)
    }
}

impl<'a> EnumWalker<'a> {
    fn get(&self) -> &'a Enum {
        &self.schema[self.id]
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
    /// Find a table by name.
    fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>>;

    /// Find a table by id.
    fn table_walker_at(&self, table_id: TableId) -> TableWalker<'_>;

    /// Find a view by index.
    fn view_walker_at(&self, index: usize) -> ViewWalker<'_>;

    /// Find a user-defined type by index.
    fn udt_walker_at(&self, index: usize) -> UserDefinedTypeWalker<'_>;

    /// Walk an enum by ID.
    fn walk_enum(&self, enum_id: EnumId) -> EnumWalker<'_>;

    /// Walk a column by id
    fn walk_column(&self, column_id: ColumnId) -> ColumnWalker<'_>;
}

impl SqlSchemaExt for SqlSchema {
    fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>> {
        Some(TableWalker {
            id: TableId(self.tables.iter().position(|table| table.name == name)? as u32),
            schema: self,
        })
    }

    fn table_walker_at(&self, id: TableId) -> TableWalker<'_> {
        TableWalker { id, schema: self }
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

    fn walk_enum(&self, id: EnumId) -> EnumWalker<'_> {
        EnumWalker { schema: self, id }
    }

    fn walk_column(&self, id: ColumnId) -> ColumnWalker<'_> {
        ColumnWalker { schema: self, id }
    }
}

/// For a slice sorted by a key K, return the contiguous range of items matching the key.
fn range_for_key<I, K>(slice: &[I], key: K, extract: fn(&I) -> K) -> Range<usize>
where
    K: Copy + Ord + PartialOrd + PartialEq,
{
    let seed = slice.binary_search_by_key(&key, extract).unwrap_or(0);
    let mut iter = slice[..seed].iter();
    let start = match iter.rposition(|i| extract(i) != key) {
        None => 0,
        Some(other) => other + 1,
    };
    let mut iter = slice[seed..].iter();
    let end = seed + iter.position(|i| extract(i) != key).unwrap_or(slice.len() - seed);
    start..end
}
