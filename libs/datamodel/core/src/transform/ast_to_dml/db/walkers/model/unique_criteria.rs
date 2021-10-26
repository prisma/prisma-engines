use crate::{
    ast,
    transform::ast_to_dml::db::{walkers::ScalarFieldWalker, ParserDatabase},
};

/// Describes any unique criteria in a model. Can either be a primary
/// key, or a unique index.
#[derive(Copy, Clone)]
pub(crate) struct UniqueCriteriaWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) fields: &'db [ast::FieldId],
    pub(crate) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> UniqueCriteriaWalker<'ast, 'db> {
    pub(crate) fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        self.fields.iter().map(move |field_id| ScalarFieldWalker {
            model_id: self.model_id,
            field_id: *field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, *field_id)],
        })
    }

    /// A model must have at least one strict unique criteria. The underlying
    /// scalar fields cannot be false or of
    pub(crate) fn is_strict_criteria(self) -> bool {
        self.fields().all(|field| !field.is_optional())
    }

    pub(crate) fn has_unsupported_fields(self) -> bool {
        self.fields().any(|field| field.is_unsupported())
    }
}
