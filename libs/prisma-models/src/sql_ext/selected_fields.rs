use crate::{AsColumn, SelectedFields};
use quaint::ast::Column;

pub trait SelectedFieldsExt {
    const RELATED_MODEL_ALIAS: &'static str = "__RelatedModel__";
    const PARENT_MODEL_ALIAS: &'static str = "__ParentModel__";

    fn columns<'a>(&'a self) -> Box<dyn Iterator<Item = Column<'_>> + 'a>;
}

impl SelectedFieldsExt for SelectedFields {
    fn columns<'a>(&'a self) -> Box<dyn Iterator<Item = Column<'_>> + 'a> {
        let scalars = self.scalar_non_list().map(|sf| sf.as_column());
        let rels = self.relation_inlined().map(|rf| rf.as_column());

        Box::new(scalars.chain(rels))
    }
}
