use std::borrow::Cow;

use crate::{
    ast,
    common::constraint_names::ConstraintNames,
    transform::ast_to_dml::db::{types::IdAttribute, ParserDatabase},
};

#[derive(Copy, Clone)]
pub(crate) struct PrimaryKeyWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(super) attribute: &'db IdAttribute<'ast>,
    pub(crate) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> PrimaryKeyWalker<'ast, 'db> {
    pub(crate) fn final_database_name(&self) -> Option<Cow<'ast, str>> {
        if !self.db.active_connector().supports_named_primary_keys() {
            return None;
        }

        Some(self.attribute.db_name.map(Cow::Borrowed).unwrap_or_else(|| {
            ConstraintNames::primary_key_name(
                self.db.walk_model(self.model_id).final_database_name(),
                self.db.active_connector(),
            )
            .into()
        }))
    }

    pub(crate) fn is_defined_on_field(&self) -> bool {
        self.attribute.source_field.is_some()
    }

    pub(crate) fn iter_ast_fields(&self) -> impl Iterator<Item = &'ast ast::Field> + '_ {
        self.attribute
            .fields
            .iter()
            .map(move |id| &self.db.ast[self.model_id][*id])
    }

    pub(crate) fn name(&self) -> Option<&'ast str> {
        self.attribute.name
    }
}
