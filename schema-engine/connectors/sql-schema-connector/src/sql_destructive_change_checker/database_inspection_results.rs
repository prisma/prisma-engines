use std::collections::HashMap;

use super::check::{Column, Table};

/// The information about the current state of the database gathered by the destructive change checker.
#[derive(Debug, Default)]
pub(crate) struct DatabaseInspectionResults {
    /// HashMap from table name to row count.
    row_counts: HashMap<Table, i64>,
    /// HashMap from (table name, column name) to non-null values count.
    value_counts: HashMap<Column, i64>,
}

impl DatabaseInspectionResults {
    pub fn get_row_count(&self, table: &Table) -> Option<i64> {
        self.row_counts.get(table).copied()
    }

    pub fn set_row_count(&mut self, table: Table, row_count: i64) {
        self.row_counts.insert(table, row_count);
    }

    pub fn get_row_and_non_null_value_count(&self, column: &Column) -> (Option<i64>, Option<i64>) {
        let table = Table::from_column(column);
        (
            self.row_counts.get(&table).copied(),
            self.value_counts.get(column).copied(),
        )
    }

    pub fn set_value_count(&mut self, column: Column, count: i64) {
        self.value_counts.insert(column, count);
    }
}
