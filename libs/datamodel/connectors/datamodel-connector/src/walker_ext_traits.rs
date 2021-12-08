use crate::constraint_names::ConstraintNames;
use crate::Connector;
use crate::{ReferentialAction, ReferentialIntegrity};
use parser_database::{ast, walkers::*};
use std::borrow::Cow;

pub trait IndexWalkerExt<'ast> {
    fn final_database_name(self, connector: &dyn Connector) -> Cow<'ast, str>;
}

impl<'ast> IndexWalkerExt<'ast> for IndexWalker<'ast, '_> {
    fn final_database_name(self, connector: &dyn Connector) -> Cow<'ast, str> {
        if let Some(mapped_name) = self.mapped_name() {
            return Cow::from(mapped_name);
        }

        let model = self.model();
        let model_db_name = model.final_database_name();
        let field_db_names: Vec<&str> = model
            .get_field_db_names(&self.fields().map(|f| f.field_id()).collect::<Vec<_>>())
            .collect();

        if self.is_unique() {
            ConstraintNames::unique_index_name(model_db_name, &field_db_names, connector).into()
        } else {
            ConstraintNames::non_unique_index_name(model_db_name, &field_db_names, connector).into()
        }
    }
}

// TODO: this lifetime doesn't make sense, it's most likely wrong.
pub trait DefaultValueExt<'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str>;
}

impl<'db> DefaultValueExt<'db> for DefaultValueWalker<'_, 'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str> {
        self.mapped_name().map(Cow::from).unwrap_or_else(|| {
            let name = ConstraintNames::default_name(
                self.field().model().final_database_name(),
                self.field().database_name(),
                connector,
            );

            Cow::from(name)
        })
    }
}

pub trait PrimaryKeyWalkerExt<'ast> {
    fn final_database_name(self, connector: &dyn Connector) -> Option<Cow<'ast, str>>;
}

impl<'ast> PrimaryKeyWalkerExt<'ast> for PrimaryKeyWalker<'ast, '_> {
    fn final_database_name(self, connector: &dyn Connector) -> Option<Cow<'ast, str>> {
        if !connector.supports_named_primary_keys() {
            return None;
        }

        Some(
            self.mapped_name().map(Cow::Borrowed).unwrap_or_else(|| {
                ConstraintNames::primary_key_name(self.model().final_database_name(), connector).into()
            }),
        )
    }
}

pub trait CompleteInlineRelationWalkerExt<'ast> {
    /// Gives the onDelete referential action of the relation. If not defined
    /// explicitly, returns the default value.
    fn on_delete(self, connector: &dyn Connector, referential_integrity: ReferentialIntegrity) -> ReferentialAction;
}

impl<'ast> CompleteInlineRelationWalkerExt<'ast> for CompleteInlineRelationWalker<'ast, '_> {
    fn on_delete(self, connector: &dyn Connector, referential_integrity: ReferentialIntegrity) -> ReferentialAction {
        use crate::ReferentialAction::*;

        self.referencing_field().explicit_on_delete().unwrap_or_else(|| {
            let supports_restrict = connector.supports_referential_action(&referential_integrity, Restrict);

            match self.referential_arity() {
                ast::FieldArity::Required if supports_restrict => Restrict,
                ast::FieldArity::Required => NoAction,
                _ => SetNull,
            }
        })
    }
}

pub trait InlineRelationWalkerExt<'ast> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str>;
}

impl<'ast> InlineRelationWalkerExt<'ast> for InlineRelationWalker<'ast, '_> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str> {
        self.foreign_key_name().map(Cow::Borrowed).unwrap_or_else(|| {
            let model_database_name = self.referencing_model().final_database_name();
            match self.referencing_fields() {
                ReferencingFields::Concrete(fields) => {
                    let field_names: Vec<&str> = fields.map(|f| f.database_name()).collect();
                    ConstraintNames::foreign_key_constraint_name(model_database_name, &field_names, connector).into()
                }
                ReferencingFields::Inferred(fields) => {
                    let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
                    ConstraintNames::foreign_key_constraint_name(model_database_name, &field_names, connector).into()
                }
                ReferencingFields::NA => unreachable!(),
            }
        })
    }
}
