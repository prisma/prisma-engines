use crate::{AsColumn, AsColumns, SelectedFields};
use quaint::ast::Column;

pub trait SelectedFieldsExt {
    fn columns<'a>(&'a self) -> Box<dyn Iterator<Item = Column<'static>> + 'a>;
}

impl SelectedFieldsExt for SelectedFields {
    fn columns<'a>(&'a self) -> Box<dyn Iterator<Item = Column<'static>> + 'a> {
        let scalar = self.scalar_fields().map(|f| f.as_column());
        let relation = self.relation_fields().flat_map(|rf| rf.as_columns());

        Box::new(scalar.chain(relation))
    }
}
