use crate::{
    ast,
    common::constraint_names::ConstraintNames,
    transform::ast_to_dml::db::{types::IndexAttribute, ParserDatabase},
};
use std::borrow::Cow;

use super::ScalarFieldWalker;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) struct IndexWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) index: Option<&'ast ast::Attribute>,
    pub(crate) db: &'db ParserDatabase<'ast>,
    pub(crate) index_attribute: &'db IndexAttribute<'ast>,
}

impl<'ast, 'db> IndexWalker<'ast, 'db> {
    pub(crate) fn final_database_name(self) -> Cow<'ast, str> {
        if let Some(mapped_name) = self.index_attribute.db_name.as_ref() {
            return mapped_name.clone(); // :( :( :(
        }

        let model = self.db.walk_model(self.model_id);
        let model_db_name = model.final_database_name();
        let field_db_names: Vec<&str> = model.get_field_db_names(&self.index_attribute.fields).collect();

        if self.index_attribute.is_unique {
            ConstraintNames::unique_index_name(model_db_name, &field_db_names, self.db.active_connector()).into()
        } else {
            ConstraintNames::non_unique_index_name(model_db_name, &field_db_names, self.db.active_connector()).into()
        }
    }

    pub(crate) fn attribute(self) -> &'db IndexAttribute<'ast> {
        self.index_attribute
    }

    pub(crate) fn fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        self.index_attribute
            .fields
            .iter()
            .map(move |field_id| ScalarFieldWalker {
                model_id: self.model_id,
                field_id: *field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(self.model_id, *field_id)],
            })
    }

    pub(crate) fn contains_exactly_fields(
        self,
        fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>>,
    ) -> bool {
        self.index_attribute.fields.len() == fields.len() && self.fields().zip(fields).all(|(a, b)| a == b)
    }

    pub(crate) fn is_unique(self) -> bool {
        self.index_attribute.is_unique
    }
}
