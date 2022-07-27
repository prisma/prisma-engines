use super::AsColumn;
use prisma_models::Model;
use quaint::ast::{Column, Table};

pub trait AsTable {
    fn as_table(&self) -> Table<'static>;
}

impl AsTable for Model {
    fn as_table(&self) -> Table<'static> {
        let table: Table<'static> = (self.internal_data_model().db_name.clone(), self.db_name().to_string()).into();

        let id_cols: Vec<Column<'static>> = self
            .primary_identifier()
            .as_scalar_fields()
            .expect("Primary identifier has non-scalar fields.")
            .into_iter()
            .map(|sf| sf.as_column())
            .collect();

        let table = table.add_unique_index(id_cols);

        self.unique_indexes().into_iter().fold(table, |table, index| {
            let index: Vec<Column<'static>> = index.fields().iter()
                .map(|(_, field)| field.as_column())
                .collect();

            table.add_unique_index(index)
        })
    }
}
