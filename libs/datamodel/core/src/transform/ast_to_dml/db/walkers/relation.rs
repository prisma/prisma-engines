use datamodel_connector::ConnectorCapability;
use dml::relation_info::ReferentialAction;

use crate::{
    ast::{self, FieldArity},
    transform::ast_to_dml::db::{relations::Relation, ParserDatabase},
};

use super::{ModelWalker, RelationFieldWalker, ScalarFieldWalker};

/// Represents a relation that has fields and references defined in one of the
/// relation fields. Includes 1:1 and 1:n relations that are defined correctly
/// from both sides.
#[derive(Copy, Clone)]
pub(crate) struct ExplicitRelationWalker<'ast, 'db> {
    pub(crate) side_a: (ast::ModelId, ast::FieldId),
    pub(crate) side_b: (ast::ModelId, ast::FieldId),
    #[allow(dead_code)]
    pub(crate) relation: &'db Relation<'ast>,
    pub(crate) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> ExplicitRelationWalker<'ast, 'db> {
    /// The model that defines the relation fields and actions.
    pub(crate) fn referencing_model(&self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.side_a.0,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.side_a.0],
        }
    }

    /// The implicit relation side.
    pub(crate) fn referenced_model(&self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.side_b.0,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.side_b.0],
        }
    }

    pub(crate) fn referencing_field(&self) -> RelationFieldWalker<'ast, 'db> {
        RelationFieldWalker {
            model_id: self.side_a.0,
            field_id: self.side_a.1,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.side_a.0, self.side_a.1)],
        }
    }

    /// The scalar fields defining the relation on the referenced model.
    pub(crate) fn referenced_fields(&'db self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let f = move |field_id: &ast::FieldId| {
            let model_id = self.referenced_model().model_id;

            ScalarFieldWalker {
                model_id,
                field_id: *field_id,
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
    pub(crate) fn referencing_fields(&'db self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let f = move |field_id: &ast::FieldId| {
            let model_id = self.referencing_model().model_id;

            ScalarFieldWalker {
                model_id,
                field_id: *field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            }
        };

        match self.referencing_field().relation_field.fields.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
        }
    }

    /// True if the relation uses more than one scalar field as the key.
    pub(crate) fn is_compound(&self) -> bool {
        self.referencing_fields().len() > 1
    }

    /// Gives the onUpdate referential action of the relation. If not defined
    /// explicitly, returns the default value.
    pub(crate) fn on_update(&self) -> ReferentialAction {
        use ReferentialAction::*;

        self.referencing_field().attributes().on_update.unwrap_or_else(|| {
            let uses_foreign_keys = self
                .db
                .active_connector()
                .has_capability(ConnectorCapability::ForeignKeys);

            match self.referential_arity() {
                _ if uses_foreign_keys => Cascade,
                FieldArity::Required => NoAction,
                _ => SetNull,
            }
        })
    }

    /// Gives the onDelete referential action of the relation. If not defined
    /// explicitly, returns the default value.
    pub(crate) fn on_delete(&self) -> ReferentialAction {
        use ReferentialAction::*;

        self.referencing_field().attributes().on_delete.unwrap_or_else(|| {
            let supports_restrict = self.db.active_connector().supports_referential_action(Restrict);

            match self.referential_arity() {
                FieldArity::Required if supports_restrict => Restrict,
                FieldArity::Required => NoAction,
                _ => SetNull,
            }
        })
    }

    /// Prisma allows setting the relation field as optional, even if one of the
    /// underlying scalar fields is required. For the purpose of referential
    /// actions, we count the relation field required if any of the underlying
    /// fields is required.
    pub(crate) fn referential_arity(&self) -> FieldArity {
        self.referencing_field().referential_arity()
    }
}
