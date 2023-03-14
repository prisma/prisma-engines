use crate::{
    types::FieldWithArgs,
    walkers::{IndexFieldWalker, ScalarFieldWalker},
    ParserDatabase,
};

/// Describes any unique criteria in a model. Can either be a primary
/// key, or a unique index.
#[derive(Copy, Clone)]
pub struct UniqueCriteriaWalker<'db> {
    pub(crate) fields: &'db [FieldWithArgs],
    pub(crate) db: &'db ParserDatabase,
}

impl<'db> UniqueCriteriaWalker<'db> {
    pub fn fields(self) -> impl ExactSizeIterator<Item = IndexFieldWalker<'db>> + 'db {
        self.fields.iter().map(move |field| match field.path.field_in_index() {
            either::Either::Left(id) => IndexFieldWalker::new(self.db.walk(id)),
            either::Either::Right(id) => IndexFieldWalker::new(self.db.walk(id)),
        })
    }

    pub fn is_strict_criteria(self) -> bool {
        !self.has_optional_fields() && !self.has_unsupported_fields()
    }

    pub(crate) fn has_optional_fields(self) -> bool {
        self.fields().any(|field| field.is_optional())
    }

    pub fn contains_exactly_fields(self, fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'db>>) -> bool {
        if self.fields().len() != fields.len() {
            return false;
        }

        self.fields()
            .zip(fields)
            .all(|(left, right)| left.field_id() == right.field_id())
    }

    pub fn has_unsupported_fields(self) -> bool {
        self.fields().any(|field| field.is_unsupported())
    }
}
