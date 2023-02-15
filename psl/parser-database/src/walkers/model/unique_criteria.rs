use crate::{
    ast,
    types::FieldWithArgs,
    walkers::{IndexFieldWalker, ScalarFieldId, ScalarFieldWalker},
    ParserDatabase,
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
    pub fn fields(self) -> impl ExactSizeIterator<Item = IndexFieldWalker<'db>> + 'db {
        self.fields.iter().map(move |field| {
            let field_id = field.path.field_in_index();

            match field.path.type_holding_the_indexed_field() {
                None => {
                    let walker = self.db.walk(ScalarFieldId(self.model_id, field_id));
                    IndexFieldWalker::new(walker)
                }
                Some(ctid) => {
                    let walker = self.db.walk((ctid, field_id));
                    IndexFieldWalker::new(walker)
                }
            }
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
