use crate::types::FieldWithArgs;
use crate::{
    ast,
    {walkers::ScalarFieldWalker, ParserDatabase},
};

/// Describes any unique criteria in a model. Can either be a primary
/// key, or a unique index.
#[derive(Copy, Clone)]
pub struct UniqueCriteriaWalker<'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) fields: &'db [FieldWithArgs],
    pub(crate) db: &'db ParserDatabase,
}

impl<'db> UniqueCriteriaWalker<'db> {
    pub fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + 'db {
        self.fields.iter().map(move |field| ScalarFieldWalker {
            model_id: self.model_id,
            field_id: field.field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, field.field_id)],
        })
    }

    pub fn is_strict_criteria(self) -> bool {
        !self.has_optional_fields() && !self.has_unsupported_fields()
    }

    pub(crate) fn has_optional_fields(self) -> bool {
        self.fields().any(|field| field.is_optional())
    }

    pub fn has_unsupported_fields(self) -> bool {
        self.fields().any(|field| field.is_unsupported())
    }
}
