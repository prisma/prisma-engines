use crate::{InternalDataModelWeakRef, Relation, RelationRef};
use once_cell::sync::OnceCell;
use psl::schema_ast::ast;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct RelationBuilder {
    pub id: psl::parser_database::RelationId,
    pub model_a_id: ast::ModelId,
    pub model_b_id: ast::ModelId,
}

impl RelationBuilder {
    pub fn build(self, internal_data_model: InternalDataModelWeakRef) -> RelationRef {
        let relation = Relation {
            id: self.id,
            model_a_id: self.model_a_id,
            model_b_id: self.model_b_id,
            model_a: OnceCell::new(),
            model_b: OnceCell::new(),
            field_a: OnceCell::new(),
            field_b: OnceCell::new(),
            internal_data_model,
        };

        Arc::new(relation)
    }
}
