use crate::{
    ast,
    transform::ast_to_dml::db::{walkers::ScalarFieldWalker, ParserDatabase},
};

#[derive(Copy, Clone)]
pub(crate) struct UniqueCriteriaWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) fields: &'db [ast::FieldId],
    pub(crate) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> UniqueCriteriaWalker<'ast, 'db> {
    pub(crate) fn fields(&'db self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        self.fields.iter().map(move |field_id| ScalarFieldWalker {
            model_id: self.model_id,
            field_id: *field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, *field_id)],
        })
    }
}
