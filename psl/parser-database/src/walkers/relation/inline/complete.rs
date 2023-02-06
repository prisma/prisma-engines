use schema_ast::ast;

use crate::{
    walkers::{ModelWalker, RelationFieldWalker, ScalarFieldWalker},
    ParserDatabase, ReferentialAction,
};

/// Represents a relation that has fields and references defined in one of the
/// relation fields. Includes 1:1 and 1:n relations that are defined from both sides.
#[derive(Copy, Clone)]
pub struct CompleteInlineRelationWalker<'db> {
    pub(crate) side_a: (ast::ModelId, ast::FieldId),
    pub(crate) side_b: (ast::ModelId, ast::FieldId),
    pub(crate) db: &'db ParserDatabase,
}

#[allow(missing_docs)]
impl<'db> CompleteInlineRelationWalker<'db> {
    /// The model that defines the relation fields and actions.
    pub fn referencing_model(self) -> ModelWalker<'db> {
        self.db.walk(self.side_a.0)
    }

    /// The implicit relation side.
    pub fn referenced_model(self) -> ModelWalker<'db> {
        self.db.walk(self.side_b.0)
    }

    pub fn referencing_field(self) -> RelationFieldWalker<'db> {
        RelationFieldWalker {
            id: crate::walkers::RelationFieldId(self.side_a.0, self.side_a.1),
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.side_a.0, self.side_a.1)],
        }
    }

    pub fn referenced_field(self) -> RelationFieldWalker<'db> {
        RelationFieldWalker {
            id: crate::walkers::RelationFieldId(self.side_b.0, self.side_b.1),
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.side_b.0, self.side_b.1)],
        }
    }

    /// The scalar fields defining the relation on the referenced model.
    pub fn referenced_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + 'db {
        let f = move |field_id: &ast::FieldId| {
            let model_id = self.referenced_model().id;

            ScalarFieldWalker {
                id: crate::walkers::ScalarFieldId(model_id, *field_id),
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            }
        };

        match self.referencing_field().relation_field.references.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
        }
    }

    /// The scalar fields on the defining the relation on the referencing model.
    pub fn referencing_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + 'db {
        let f = move |field_id: &ast::FieldId| {
            let model_id = self.referencing_model().id;

            ScalarFieldWalker {
                id: crate::walkers::ScalarFieldId(model_id, *field_id),
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            }
        };

        match self.referencing_field().relation_field.fields.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
        }
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

    /// Prisma allows setting the relation field as optional, even if one of the
    /// underlying scalar fields is required. For the purpose of referential
    /// actions, we count the relation field required if any of the underlying
    /// fields is required.
    pub fn referential_arity(self) -> ast::FieldArity {
        self.referencing_field().referential_arity()
    }
}
