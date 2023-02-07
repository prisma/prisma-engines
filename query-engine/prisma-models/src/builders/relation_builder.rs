use crate::{InternalDataModelWeakRef, Relation, RelationLinkManifestation, RelationRef};
use once_cell::sync::OnceCell;
use psl::{datamodel_connector::RelationMode, schema_ast::ast};
use std::sync::Arc;

#[derive(Debug)]
pub struct RelationBuilder {
    pub name: String,
    pub manifestation: RelationLinkManifestation,
    pub model_a_id: ast::ModelId,
    pub model_b_id: ast::ModelId,
    pub relation_mode: RelationMode,
}

impl RelationBuilder {
    pub fn build(self, internal_data_model: InternalDataModelWeakRef) -> RelationRef {
        let relation = Relation {
            name: self.name,
            manifestation: self.manifestation,
            model_a_id: self.model_a_id,
            model_b_id: self.model_b_id,
            relation_mode: self.relation_mode,
            model_a: OnceCell::new(),
            model_b: OnceCell::new(),
            field_a: OnceCell::new(),
            field_b: OnceCell::new(),
            internal_data_model,
        };

        Arc::new(relation)
    }
}
