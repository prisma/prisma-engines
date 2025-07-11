use either::Either;

use crate::{
    Column, ColumnArity, ColumnType, ColumnTypeFamily, EnumWalker, TableColumnId, TableDefaultValueId,
    TableDefaultValueWalker, TableId, TableWalker, Walker,
};

use super::ColumnWalker;

/// Traverse a table column.
pub type TableColumnWalker<'a> = Walker<'a, TableColumnId>;

impl<'a> TableColumnWalker<'a> {
    fn get(self) -> &'a (TableId, Column) {
        &self.schema.table_columns[self.id.0 as usize]
    }

    /// Coarsen the walker into a generic column version.
    pub fn coarsen(self) -> ColumnWalker<'a> {
        self.walk(Either::Left(self.id))
    }

    /// The column name.
    pub fn name(self) -> &'a str {
        self.coarsen().name()
    }

    /// The nullability and arity of the column.
    pub fn arity(self) -> ColumnArity {
        self.coarsen().arity()
    }

    /// Returns whether the column has the enum default value of the given enum type.
    pub fn column_has_enum_default_value(self, enum_name: &str, value: &str) -> bool {
        self.coarsen().column_has_enum_default_value(enum_name, value)
    }

    /// Returns whether the type of the column matches the provided enum name.
    pub fn column_type_is_enum(self, enum_name: &str) -> bool {
        self.coarsen().column_type_is_enum(enum_name)
    }

    /// The type family.
    pub fn column_type_family(self) -> &'a ColumnTypeFamily {
        self.coarsen().column_type_family()
    }

    /// Extract an `Enum` column type family, or `None` if the family is something else.
    pub fn column_type_family_as_enum(self) -> Option<EnumWalker<'a>> {
        self.coarsen().column_type_family_as_enum()
    }

    /// the default value for the column.
    pub fn default(self) -> Option<TableDefaultValueWalker<'a>> {
        self.schema
            .table_default_values
            .binary_search_by_key(&self.id, |(id, _)| *id)
            .ok()
            .map(|id| self.walk(TableDefaultValueId(id as u32)))
    }

    /// The full column type.
    pub fn column_type(self) -> &'a ColumnType {
        self.coarsen().column_type()
    }

    /// The column native type.
    pub fn column_native_type<T: std::any::Any + 'static>(self) -> Option<&'a T> {
        self.coarsen().column_native_type()
    }

    /// Is this column an auto-incrementing integer?
    pub fn is_autoincrement(self) -> bool {
        self.coarsen().is_autoincrement()
    }

    /// Returns whether two columns are named the same and belong to the same table.
    pub fn is_same_column(self, other: TableColumnWalker<'_>) -> bool {
        self.name() == other.name()
            && self.table().name() == other.table().name()
            && self.table().namespace_id() == other.table().namespace_id()
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

    /// Description (comment) of the column.
    pub fn description(self) -> Option<&'a str> {
        self.coarsen().description()
    }
}
