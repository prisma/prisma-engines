use std::borrow::Cow;

use crate::{
    ast,
    common::constraint_names::ConstraintNames,
    transform::ast_to_dml::db::{types::IndexAttribute, ParserDatabase},
};

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) struct IndexWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) index: &'ast ast::Attribute,
    pub(crate) db: &'db ParserDatabase<'ast>,
    pub(crate) index_attribute: &'db IndexAttribute<'ast>,
}

impl<'ast, 'db> IndexWalker<'ast, 'db> {
    pub(crate) fn final_database_name(&self) -> Cow<'ast, str> {
        if let Some(mapped_name) = &self.index_attribute.db_name {
            return Cow::Borrowed(mapped_name);
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

    pub(crate) fn attribute(&self) -> &'db IndexAttribute<'ast> {
        self.index_attribute
    }
}
