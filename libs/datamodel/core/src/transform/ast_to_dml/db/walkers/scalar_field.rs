use crate::{
    ast,
    transform::ast_to_dml::db::{ParserDatabase, ScalarField},
};

use super::ModelWalker;

#[derive(Copy, Clone)]
pub(crate) struct ScalarFieldWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) field_id: ast::FieldId,
    pub(crate) db: &'db ParserDatabase<'ast>,
    pub(crate) scalar_field: &'db ScalarField<'ast>,
}

impl<'ast, 'db> PartialEq for ScalarFieldWalker<'ast, 'db> {
    fn eq(&self, other: &Self) -> bool {
        self.model_id == other.model_id && self.field_id == other.field_id
    }
}

impl<'ast, 'db> Eq for ScalarFieldWalker<'ast, 'db> {}

impl<'ast, 'db> ScalarFieldWalker<'ast, 'db> {
    #[allow(dead_code)] // we'll need this
    pub(crate) fn field_id(self) -> ast::FieldId {
        self.field_id
    }

    pub(crate) fn ast_field(self) -> &'ast ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    pub(crate) fn name(self) -> &'ast str {
        self.ast_field().name()
    }

    pub(crate) fn final_database_name(self) -> &'ast str {
        self.attributes().mapped_name.unwrap_or_else(|| self.name())
    }

    pub(crate) fn is_optional(self) -> bool {
        self.ast_field().arity.is_optional()
    }

    #[allow(dead_code)] // we'll need this
    pub(crate) fn attributes(self) -> &'db ScalarField<'ast> {
        self.scalar_field
    }

    #[allow(dead_code)] // we'll need this
    pub(crate) fn model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }

    pub(crate) fn is_unsupported(self) -> bool {
        matches!(self.ast_field().field_type, ast::FieldType::Unsupported(_, _))
    }
}
