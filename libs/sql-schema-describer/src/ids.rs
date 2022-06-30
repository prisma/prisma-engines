use crate::{Column, Enum, SqlSchema, Table};
use serde::*;
use std::ops::{Index, IndexMut};

/// The identifier for a table in a SqlSchema. Use it with the indexing syntax:
/// `let table = schema[table_id];`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TableId(pub u32);

impl Index<TableId> for SqlSchema {
    type Output = Table;

    fn index(&self, index: TableId) -> &Self::Output {
        &self.tables[index.0 as usize]
    }
}

impl IndexMut<TableId> for SqlSchema {
    fn index_mut(&mut self, index: TableId) -> &mut Self::Output {
        &mut self.tables[index.0 as usize]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColumnId(pub u32);

impl Index<ColumnId> for SqlSchema {
    type Output = (TableId, Column);

    fn index(&self, index: ColumnId) -> &Self::Output {
        &self.columns[index.0 as usize]
    }
}

impl IndexMut<ColumnId> for SqlSchema {
    fn index_mut(&mut self, index: ColumnId) -> &mut Self::Output {
        &mut self.columns[index.0 as usize]
    }
}

/// The identifier for an Index in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexId(pub TableId, pub u32);

impl Index<IndexId> for SqlSchema {
    type Output = crate::Index;

    fn index(&self, index: IndexId) -> &Self::Output {
        &self[index.0].indices[index.1 as usize]
    }
}

/// The identifier for an Index in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexFieldId(pub IndexId, pub u32);

impl Index<IndexFieldId> for SqlSchema {
    type Output = crate::IndexColumn;

    fn index(&self, index: IndexFieldId) -> &Self::Output {
        &self[index.0].columns[index.1 as usize]
    }
}

/// The identifier for a ForeignKey in the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ForeignKeyId(pub(crate) u32);
