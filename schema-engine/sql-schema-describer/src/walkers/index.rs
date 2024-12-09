use crate::{Index, IndexColumnId, IndexColumnWalker, IndexId, IndexType, TableColumnId, TableWalker, Walker};

/// Traverse an index.
pub type IndexWalker<'a> = Walker<'a, IndexId>;

impl<'a> IndexWalker<'a> {
    /// The names of the indexed columns.
    pub fn column_names(self) -> impl ExactSizeIterator<Item = &'a str> {
        self.columns().map(|c| c.as_column().name())
    }

    /// Traverse the indexed columns.
    pub fn columns(self) -> impl ExactSizeIterator<Item = IndexColumnWalker<'a>> {
        super::range_for_key(&self.schema.index_columns, self.id, |i| i.index_id)
            .map(move |idx| self.walk(IndexColumnId(idx as u32)))
    }

    /// True if index contains the given column.
    pub fn contains_column(self, column_id: TableColumnId) -> bool {
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

    /// Is this index a normal index?
    pub fn is_normal(self) -> bool {
        matches!(self.get().tpe, IndexType::Normal)
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
