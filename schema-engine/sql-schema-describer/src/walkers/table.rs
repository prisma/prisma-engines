use std::ops::Range;

use crate::{
    ForeignKeyId, ForeignKeyWalker, IndexColumnWalker, IndexId, IndexWalker, NamespaceId, Table, TableColumnId,
    TableColumnWalker, TableId, TableProperties, Walker,
};

/// Traverse a table.
pub type TableWalker<'a> = Walker<'a, TableId>;

impl<'a> TableWalker<'a> {
    /// Get a column in the table, by name.
    pub fn column(self, column_name: &str) -> Option<TableColumnWalker<'a>> {
        self.columns().find(|column| column.name() == column_name)
    }

    fn columns_range(self) -> Range<usize> {
        super::range_for_key(&self.schema.table_columns, self.id, |(tid, _)| *tid)
    }

    /// Traverse the table's columns.
    pub fn columns(self) -> impl ExactSizeIterator<Item = TableColumnWalker<'a>> {
        self.columns_range()
            .map(move |idx| self.walk(TableColumnId(idx as u32)))
    }

    /// The number of foreign key constraints on the table.
    pub fn foreign_key_count(self) -> usize {
        self.foreign_keys_range().len()
    }

    /// Traverse the indexes on the table.
    pub fn indexes(self) -> impl ExactSizeIterator<Item = IndexWalker<'a>> {
        let range = super::range_for_key(&self.schema.indexes, self.id, |idx| idx.table_id);
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
        super::range_for_key(&self.schema.foreign_keys, self.id, |fk| fk.constrained_table)
    }

    /// Try to traverse a foreign key for a single column.
    pub fn foreign_key_for_column(self, column: TableColumnId) -> Option<ForeignKeyWalker<'a>> {
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

    /// Is the table a partition table?
    pub fn is_partition(self) -> bool {
        self.table().properties.contains(TableProperties::IsPartition)
    }

    /// Does the table have subclasses?
    pub fn has_subclass(self) -> bool {
        self.table().properties.contains(TableProperties::HasSubclass)
    }

    /// Does the table have row level security enabled?
    pub fn has_row_level_security(self) -> bool {
        self.table().properties.contains(TableProperties::HasRowLevelSecurity)
    }

    /// Does the table have check constraints?
    pub fn has_check_constraints(self) -> bool {
        self.schema
            .check_constraints
            .binary_search_by_key(&self.id, |(id, _)| *id)
            .is_ok()
    }

    /// Returns whether two tables have same properties, belong to the same table, but have different name.
    pub fn is_renamed_table(self, other: TableWalker<'_>) -> bool {
        self.name() != other.name()
            && self.table().namespace_id == other.table().namespace_id
            && self.table().properties == other.table().properties
            && self.primary_key().unwrap().name() == other.primary_key().unwrap().name()
    }

    /// The check constraint names for the table.
    pub fn check_constraints(self) -> impl ExactSizeIterator<Item = &'a str> {
        let low = self.schema.check_constraints.partition_point(|(id, _)| *id < self.id);
        let high = self.schema.check_constraints[low..].partition_point(|(id, _)| *id <= self.id);

        self.schema.check_constraints[low..low + high]
            .iter()
            .map(|(_, name)| name.as_str())
    }

    /// Description (comment) of the table.
    pub fn description(self) -> Option<&'a str> {
        self.table().description.as_deref()
    }

    /// Reference to the underlying `Table` struct.
    fn table(self) -> &'a Table {
        &self.schema.tables[self.id.0 as usize]
    }
}
