use crate::{
    walkers::{ModelWalker, RelationFieldId, RelationFieldWalker, ScalarFieldWalker},
    ParserDatabase, ReferentialAction,
};
use diagnostics::Span;
use schema_ast::ast;

/// Represents a relation that has fields and references defined in one of the
/// relation fields. Includes 1:1 and 1:n relations that are defined from both sides.
#[derive(Copy, Clone)]
pub struct CompleteInlineRelationWalker<'db> {
    pub(crate) side_a: RelationFieldId,
    pub(crate) side_b: RelationFieldId,
    /// The parser database being traversed.
    pub db: &'db ParserDatabase,
}

#[allow(missing_docs)]
impl<'db> CompleteInlineRelationWalker<'db> {
    /// The model that defines the relation fields and actions.
    pub fn referencing_model(self) -> ModelWalker<'db> {
        self.db.walk(self.side_a).model()
    }

    /// The implicit relation side.
    pub fn referenced_model(self) -> ModelWalker<'db> {
        self.db.walk(self.side_b).model()
    }

    pub fn referencing_field(self) -> RelationFieldWalker<'db> {
        self.db.walk(self.side_a)
    }

    pub fn referenced_field(self) -> RelationFieldWalker<'db> {
        self.db.walk(self.side_b)
    }

    /// The scalar fields defining the relation on the referenced model.
    pub fn referenced_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + 'db {
        (match self.referencing_field().attributes().references.as_ref() {
            Some(references) => references.as_slice(),
            None => &[],
        })
        .iter()
        .map(|id| self.db.walk(*id))
    }

    /// The scalar fields on the defining the relation on the referencing model.
    pub fn referencing_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + 'db {
        (match self.referencing_field().attributes().fields.as_ref() {
            Some(references) => references.as_slice(),
            None => &[],
        })
        .iter()
        .map(|id| self.db.walk(*id))
    }

    /// Gives the onUpdate referential action of the relation. If not defined
    /// explicitly, returns the default value.
    pub fn on_update(self) -> ReferentialAction {
        use ReferentialAction::*;

        self.referencing_field()
            .attributes()
            .on_update
            .map(|(action, _)| action)
            .unwrap_or(Cascade)
    }

    pub fn on_update_span(self) -> Option<Span> {
        self.referencing_field().attributes().on_update.map(|(_, span)| span)
    }

    /// Prisma allows setting the relation field as optional, even if one of the
    /// underlying scalar fields is required. For the purpose of referential
    /// actions, we count the relation field required if any of the underlying
    /// fields is required.
    pub fn referential_arity(self) -> ast::FieldArity {
        self.referencing_field().referential_arity()
    }
}
