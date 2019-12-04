use crate::Model;
use quaint::ast::Table;

pub trait AsTable {
    fn as_table(&self) -> Table<'static>;
}

impl AsTable for Model {
    fn as_table(&self) -> Table<'static> {
        (self.internal_data_model().db_name.clone(), self.db_name().to_string()).into()
    }
}
