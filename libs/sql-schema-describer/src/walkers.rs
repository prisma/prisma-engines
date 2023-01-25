//! Functions and types for conveniently traversing and querying a SqlSchema.

#![deny(missing_docs)]

use crate::{
    ids::*, Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultKind, DefaultValue, Enum, ForeignKey,
    ForeignKeyAction, ForeignKeyColumn, Index, IndexColumn, IndexType, SQLSortOrder, SqlSchema, Table, UserDefinedType,
    View,
};
use prisma_value::PrismaValue;
use std::ops::Range;

/// A generic reference to a schema item. It holds a reference to the schema so it can offer a
/// convenient API based on the Id type.
#[derive(Clone, Copy)]
pub struct Walker<'a, Id> {
    /// The identifier.
    pub id: Id,
    /// The schema for which the identifier is valid.
    pub schema: &'a SqlSchema,
}

impl<I: std::fmt::Debug> std::fmt::Debug for Walker<'_, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("id", &self.id)
            .finish()
    }
}

impl<'a, Id> Walker<'a, Id> {
    /// Jump to the item identified by `other_id`.
    pub fn walk<I>(self, other_id: I) -> Walker<'a, I> {
        self.schema.walk(other_id)
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

/// Traverse an enum variant.
pub type EnumVariantWalker<'a> = Walker<'a, EnumVariantId>;

/// Traverse an index.
pub type IndexWalker<'a> = Walker<'a, IndexId>;

/// Traverse a specific column inside an index.
pub type IndexColumnWalker<'a> = Walker<'a, IndexColumnId>;

/// Traverse a namespace
pub type NamespaceWalker<'a> = Walker<'a, NamespaceId>;

/// Traverse a user-defined type
pub type UserDefinedTypeWalker<'a> = Walker<'a, UdtId>;

/// Traverse a view
pub type ViewWalker<'a> = Walker<'a, ViewId>;

/// Traverse a default value
pub type DefaultValueWalker<'a> = Walker<'a, DefaultValueId>;

impl<'a> DefaultValueWalker<'a> {
    /// Return a value if a constant.
    pub fn as_value(self) -> Option<&'a PrismaValue> {
        match self.kind() {
            DefaultKind::Value(ref v) => Some(v),
            _ => None,
        }
    }

    /// If the value is a squence, return it
    pub fn as_sequence(&self) -> Option<&str> {
        match self.kind() {
            DefaultKind::Sequence(name) => Some(name),
            _ => None,
        }
    }

    /// True if a constant value
    pub fn is_value(&self) -> bool {
        matches!(self.kind(), DefaultKind::Value(_))
    }

    /// True if `now()`
    pub fn is_now(&self) -> bool {
        matches!(self.kind(), DefaultKind::Now)
    }

    /// True if referencing a sequence
    pub fn is_sequence(&self) -> bool {
        matches!(self.kind(), DefaultKind::Sequence(_))
    }

    /// True if value generation is handled in the database
    pub fn is_db_generated(&self) -> bool {
        matches!(self.kind(), DefaultKind::DbGenerated(_))
    }

    /// The value kind enumerator
    pub fn kind(self) -> &'a DefaultKind {
        &self.get().1.kind
    }

    /// The name of the default value constraint.
    pub fn constraint_name(self) -> Option<&'a str> {
        self.get().1.constraint_name.as_deref()
    }

    /// The column where the default value is located.
    pub fn column(&self) -> ColumnWalker<'a> {
        self.walk(self.get().0)
    }

    /// The default value data
    pub fn inner(self) -> &'a DefaultValue {
        &self.schema.default_values[self.id.0 as usize].1
    }

    fn get(self) -> &'a (ColumnId, DefaultValue) {
        &self.schema.default_values[self.id.0 as usize]
    }
}

impl<'a> ColumnWalker<'a> {
    /// The nullability and arity of the column.
    pub fn arity(self) -> ColumnArity {
        self.get().1.tpe.arity
    }

    fn get(self) -> &'a (TableId, Column) {
        &self.schema.columns[self.id.0 as usize]
    }

    /// Returns whether the column has the enum default value of the given enum type.
    pub fn column_has_enum_default_value(self, enum_name: &str, value: &str) -> bool {
        self.column_type_family_as_enum().map(|enm| enm.name()) == Some(enum_name)
            && self
                .default()
                .and_then(|default| default.as_value())
                .and_then(|value| value.as_enum_value())
                == Some(value)
    }

    /// Returns whether the type of the column matches the provided enum name.
    pub fn column_type_is_enum(self, enum_name: &str) -> bool {
        self.column_type_family_as_enum()
            .map(|enm| enm.name() == enum_name)
            .unwrap_or(false)
    }

    /// The type family.
    pub fn column_type_family(self) -> &'a ColumnTypeFamily {
        &self.get().1.tpe.family
    }

    /// Extract an `Enum` column type family, or `None` if the family is something else.
    pub fn column_type_family_as_enum(self) -> Option<EnumWalker<'a>> {
        self.column_type_family().as_enum().map(|enum_id| self.walk(enum_id))
    }

    /// The column name.
    pub fn name(self) -> &'a str {
        &self.get().1.name
    }

    /// The default value for the column.
    pub fn default(self) -> Option<DefaultValueWalker<'a>> {
        self.get()
            .1
            .default_value_id
            .map(move |default_id| self.walk(default_id))
    }

    /// The full column type.
    pub fn column_type(self) -> &'a ColumnType {
        &self.get().1.tpe
    }

    /// The column native type.
    pub fn column_native_type<T: std::any::Any + 'static>(self) -> Option<&'a T> {
        self.column_type().native_type.as_ref().map(|nt| nt.downcast_ref())
    }

    /// Is this column an auto-incrementing integer?
    pub fn is_autoincrement(self) -> bool {
        self.get().1.auto_increment
    }

    /// Is this column indexed by a secondary index??
    pub fn is_part_of_secondary_index(self) -> bool {
        self.table().indexes().any(|idx| idx.contains_column(self.id))
    }

    /// Is this column a part of the table's primary key?
    pub fn is_part_of_primary_key(self) -> bool {
        match self.table().primary_key() {
            Some(pk) => pk.contains_column(self.id),
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
    pub fn is_same_column(self, other: ColumnWalker<'_>) -> bool {
        self.name() == other.name() && self.table().name() == other.table().name()
    }

    /// Returns whether this column is the primary key. If it is only part of the primary key, this will return false.
    pub fn is_single_primary_key(self) -> bool {
        self.table()
            .primary_key()
            .map(|pk| pk.columns().len() == 1 && pk.columns().next().map(|c| c.name() == self.name()).unwrap_or(false))
            .unwrap_or(false)
    }

    /// Traverse to the column's table.
    pub fn table(self) -> TableWalker<'a> {
        self.walk(self.get().0)
    }
}

impl<'a> ViewWalker<'a> {
    /// The name of the view
    pub fn name(self) -> &'a str {
        &self.view().name
    }

    /// The SQL definition of the view
    pub fn definition(self) -> Option<&'a str> {
        self.view().definition.as_deref()
    }

    /// The namespace of the view
    pub fn namespace(self) -> Option<&'a str> {
        self.schema
            .namespaces
            .get(self.view().namespace_id.0 as usize)
            .map(|s| s.as_str())
    }

    fn view(self) -> &'a View {
        &self.schema.views[self.id.0 as usize]
    }
}

impl<'a> UserDefinedTypeWalker<'a> {
    /// The name of the type
    pub fn name(self) -> &'a str {
        &self.udt().name
    }

    /// The SQL definition of the type
    pub fn definition(self) -> Option<&'a str> {
        self.udt().definition.as_deref()
    }

    /// The namespace of the type
    pub fn namespace(self) -> Option<&'a str> {
        self.schema
            .namespaces
            .get(self.udt().namespace_id.0 as usize)
            .map(|s| s.as_str())
    }

    fn udt(self) -> &'a UserDefinedType {
        &self.schema.user_defined_types[self.id.0 as usize]
    }
}

impl<'a> TableWalker<'a> {
    /// Get a column in the table, by name.
    pub fn column(self, column_name: &str) -> Option<ColumnWalker<'a>> {
        self.columns().find(|column| column.name() == column_name)
    }

    /// Get a column in the table, by name.
    pub fn column_case_insensitive(self, column_name: &str) -> Option<ColumnWalker<'a>> {
        self.columns().find(|column| column.name() == column_name)
    }

    fn columns_range(self) -> Range<usize> {
        range_for_key(&self.schema.columns, self.id, |(tid, _)| *tid)
    }

    /// Traverse the table's columns.
    pub fn columns(self) -> impl ExactSizeIterator<Item = ColumnWalker<'a>> {
        self.columns_range()
            .into_iter()
            .map(move |idx| self.walk(ColumnId(idx as u32)))
    }

    /// The number of foreign key constraints on the table.
    pub fn foreign_key_count(self) -> usize {
        self.foreign_keys_range().into_iter().len()
    }

    /// Traverse the indexes on the table.
    pub fn indexes(self) -> impl ExactSizeIterator<Item = IndexWalker<'a>> {
        let range = range_for_key(&self.schema.indexes, self.id, |idx| idx.table_id);
        range.map(move |idx| self.walk(IndexId(idx as u32)))
    }

    /// Traverse the foreign keys on the table.
    pub fn foreign_keys(self) -> impl ExactSizeIterator<Item = ForeignKeyWalker<'a>> {
        self.foreign_keys_range()
            .map(move |id| self.walk(ForeignKeyId(id as u32)))
    }

    /// Traverse foreign keys from other tables, referencing current table.
    pub fn referencing_foreign_keys(self) -> impl Iterator<Item = ForeignKeyWalker<'a>> {
        self.schema
            .table_walkers()
            .filter(move |t| t.id != self.id)
            .flat_map(|t| t.foreign_keys())
            .filter(move |fk| fk.referenced_table().id == self.id)
    }

    /// The table name.
    pub fn name(self) -> &'a str {
        &self.table().name
    }

    fn foreign_keys_range(self) -> Range<usize> {
        range_for_key(&self.schema.foreign_keys, self.id, |fk| fk.constrained_table)
    }

    /// Try to traverse a foreign key for a single column.
    pub fn foreign_key_for_column(self, column: ColumnId) -> Option<ForeignKeyWalker<'a>> {
        self.foreign_keys().find(|fk| {
            let cols = fk.columns();
            cols.len() == 1 && cols[0].constrained_column == column
        })
    }

    /// The namespace the table belongs to, if defined.
    pub fn namespace(self) -> Option<&'a str> {
        self.schema
            .namespaces
            .get(self.table().namespace_id.0 as usize)
            .map(|s| s.as_str())
    }

    /// The namespace the table belongs to.
    pub fn namespace_id(self) -> NamespaceId {
        self.table().namespace_id
    }

    /// Traverse to the primary key of the table.
    pub fn primary_key(self) -> Option<IndexWalker<'a>> {
        self.indexes().find(|idx| idx.is_primary_key())
    }

    /// The columns that are part of the primary keys.
    pub fn primary_key_columns(self) -> Option<impl ExactSizeIterator<Item = IndexColumnWalker<'a>>> {
        self.primary_key().map(|pk| pk.columns())
    }

    /// How many columns are in the primary key? Returns 0 in the absence of a pk.
    pub fn primary_key_columns_count(self) -> usize {
        self.primary_key_columns().map(|cols| cols.len()).unwrap_or(0)
    }

    /// Reference to the underlying `Table` struct.
    fn table(self) -> &'a Table {
        &self.schema.tables[self.id.0 as usize]
    }
}

impl<'schema> ForeignKeyWalker<'schema> {
    fn columns(self) -> &'schema [ForeignKeyColumn] {
        let range = range_for_key(&self.schema.foreign_key_columns, self.id, |col| col.foreign_key_id);
        &self.schema.foreign_key_columns[range]
    }

    /// The foreign key columns on the referencing table.
    pub fn constrained_columns(self) -> impl ExactSizeIterator<Item = ColumnWalker<'schema>> {
        self.columns().iter().map(move |col| self.walk(col.constrained_column))
    }

    /// The name of the foreign key constraint.
    pub fn constraint_name(self) -> Option<&'schema str> {
        self.foreign_key().constraint_name.as_deref()
    }

    fn foreign_key(self) -> &'schema ForeignKey {
        &self.schema.foreign_keys[self.id.0 as usize]
    }

    /// The `ON DELETE` behaviour of the foreign key.
    pub fn on_delete_action(self) -> ForeignKeyAction {
        self.foreign_key().on_delete_action
    }

    /// The `ON UPDATE` behaviour of the foreign key.
    pub fn on_update_action(self) -> ForeignKeyAction {
        self.foreign_key().on_update_action
    }

    /// The columns referenced by the foreign key on the referenced table.
    pub fn referenced_columns(self) -> impl ExactSizeIterator<Item = ColumnWalker<'schema>> {
        self.columns().iter().map(move |col| self.walk(col.referenced_column))
    }

    /// The table the foreign key "points to".
    pub fn referenced_table_name(self) -> &'schema str {
        self.referenced_table().name()
    }

    /// The table the foreign key "points to".
    pub fn referenced_table(self) -> TableWalker<'schema> {
        self.walk(self.foreign_key().referenced_table)
    }

    /// Traverse to the referencing/constrained table.
    pub fn table(self) -> TableWalker<'schema> {
        self.walk(self.foreign_key().constrained_table)
    }

    /// True if relation is back to the same table.
    pub fn is_self_relation(self) -> bool {
        let fk = self.foreign_key();
        fk.constrained_table == fk.referenced_table
    }
}

impl<'a> IndexColumnWalker<'a> {
    /// Get the index column data.
    pub fn get(self) -> &'a IndexColumn {
        &self.schema.index_columns[self.id.0 as usize]
    }

    /// The name of the column.
    pub fn name(self) -> &'a str {
        self.as_column().name()
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
    pub fn table(self) -> TableWalker<'a> {
        self.index().table()
    }

    /// The index of the column.
    pub fn index(self) -> IndexWalker<'a> {
        self.walk(self.get().index_id)
    }

    /// Convert to a normal column walker, losing the possible index arguments.
    pub fn as_column(self) -> ColumnWalker<'a> {
        self.walk(self.get().column_id)
    }
}

impl<'a> IndexWalker<'a> {
    /// The names of the indexed columns.
    pub fn column_names(self) -> impl ExactSizeIterator<Item = &'a str> {
        self.columns().map(|c| c.as_column().name())
    }

    /// Traverse the indexed columns.
    pub fn columns(self) -> impl ExactSizeIterator<Item = IndexColumnWalker<'a>> {
        range_for_key(&self.schema.index_columns, self.id, |i| i.index_id)
            .map(move |idx| self.walk(IndexColumnId(idx as u32)))
    }

    /// True if index contains the given column.
    pub fn contains_column(self, column_id: ColumnId) -> bool {
        self.columns().any(|column| column.as_column().id == column_id)
    }

    fn get(self) -> &'a Index {
        &self.schema.indexes[self.id.0 as usize]
    }

    /// The IndexType
    pub fn index_type(self) -> IndexType {
        self.get().tpe
    }

    /// Is this index the primary key of the table?
    pub fn is_primary_key(self) -> bool {
        matches!(self.get().tpe, IndexType::PrimaryKey)
    }

    /// Is this index a unique constraint? NB: This will return `false` for the primary key.
    pub fn is_unique(self) -> bool {
        matches!(self.get().tpe, IndexType::Unique)
    }

    /// The name of the index.
    pub fn name(self) -> &'a str {
        &self.get().index_name
    }

    /// Traverse to the table of the index.
    pub fn table(self) -> TableWalker<'a> {
        self.walk(self.get().table_id)
    }
}

impl<'a> EnumWalker<'a> {
    fn get(self) -> &'a Enum {
        &self.schema.enums[self.id.0 as usize]
    }

    /// The namespace the enum belongs to, if defined.
    pub fn namespace(self) -> Option<&'a str> {
        self.schema
            .namespaces
            .get(self.get().namespace_id.0 as usize)
            .map(|s| s.as_str())
    }

    /// The name of the enum. This is a made up name on MySQL.
    pub fn name(self) -> &'a str {
        &self.get().name
    }

    /// The variants of the enum
    pub fn variants(self) -> impl ExactSizeIterator<Item = EnumVariantWalker<'a>> {
        range_for_key(&self.schema.enum_variants, self.id, |variant| variant.enum_id)
            .map(move |idx| self.walk(EnumVariantId(idx as u32)))
    }

    /// The names of the variants of the enum
    pub fn values(self) -> impl ExactSizeIterator<Item = &'a str> {
        range_for_key(&self.schema.enum_variants, self.id, |variant| variant.enum_id)
            .map(move |idx| self.schema.enum_variants[idx].variant_name.as_str())
    }
}

impl<'a> EnumVariantWalker<'a> {
    fn get(self) -> &'a super::EnumVariant {
        &self.schema.enum_variants[self.id.0 as usize]
    }

    /// The parent enum.
    pub fn r#enum(self) -> EnumWalker<'a> {
        self.walk(self.get().enum_id)
    }

    /// The variant itself.
    pub fn name(self) -> &'a str {
        &self.get().variant_name
    }
}

impl<'a> NamespaceWalker<'a> {
    /// The namespace name.
    pub fn name(self) -> &'a str {
        &self.schema.namespaces[self.id.0 as usize]
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
