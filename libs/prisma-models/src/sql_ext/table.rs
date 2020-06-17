use super::AsColumn;
use crate::Model;
use quaint::ast::{Column, Table};

pub trait AsTable {
    fn as_table(&self) -> Table<'static>;
}

impl AsTable for Model {
    fn as_table(&self) -> Table<'static> {
        let table: Table<'static> = (self.internal_data_model().db_name.clone(), self.db_name().to_string()).into();

        self.unique_indexes().into_iter().fold(table, |table, index| {
            let index: Vec<Column<'static>> = index.fields().iter().map(AsColumn::as_column).collect();
            table.add_unique_index(index)
        })
    }
}
