use crate::{Column, Enum, SqlSchema, Table};
use std::ops::Index;

/// The identifier for a table in a SqlSchema. Use it with the indexing syntax:
/// `let table = schema[table_id];`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableId(pub(crate) u32);

impl Index<TableId> for SqlSchema {
    type Output = Table;

    fn index(&self, index: TableId) -> &Self::Output {
        &self.tables[index.0 as usize]
    }
}

/// The identifier for an enum in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnumId(pub(crate) u32);

impl Index<EnumId> for SqlSchema {
    type Output = Enum;

    fn index(&self, index: EnumId) -> &Self::Output {
        &self.enums[index.0 as usize]
    }
}

/// The identifier for a column in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnId(pub(crate) u32);

impl Index<ColumnId> for Table {
    type Output = Column;

    fn index(&self, index: ColumnId) -> &Self::Output {
        &self.columns[index.0 as usize]
    }
}
