use crate::{IndexColumn, IndexColumnId, IndexWalker, SQLSortOrder, TableColumnWalker, TableWalker, Walker};

/// Traverse a specific column inside an index.
pub type IndexColumnWalker<'a> = Walker<'a, IndexColumnId>;

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
    pub fn as_column(self) -> TableColumnWalker<'a> {
        self.walk(self.get().column_id)
    }
}
