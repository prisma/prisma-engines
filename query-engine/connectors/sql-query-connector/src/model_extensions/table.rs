use super::AsColumns;
use prisma_models::Model;
use quaint::ast::{Column, Table};

pub trait AsTable {
    fn as_table(&self) -> Table<'static>;
}

impl AsTable for Model {
    fn as_table(&self) -> Table<'static> {
        let table: Table<'static> = match self.db_name_with_schema() {
            (Some(s), t) => (s, t).into(),
            (None, t) => t.into(),
        };

        let id_cols: Vec<Column<'static>> = self
            .primary_identifier()
            .as_scalar_fields()
            .expect("Primary identifier has non-scalar fields.")
            .as_columns()
            .collect();

        let table = table.add_unique_index(id_cols);

        self.unique_indexes().into_iter().fold(table, |table, index| {
            let index: Vec<Column<'static>> = index.fields().as_columns().collect();
            table.add_unique_index(index)
        })
    }
}
