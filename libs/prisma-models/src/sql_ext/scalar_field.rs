use crate::ScalarField;
use crate::ScalarListTable;

pub trait ScalarFieldExt {
    fn scalar_list_table(&self) -> ScalarListTable;
}

impl ScalarFieldExt for ScalarField {
    fn scalar_list_table(&self) -> ScalarListTable {
        ScalarListTable::new(self)
    }
}
