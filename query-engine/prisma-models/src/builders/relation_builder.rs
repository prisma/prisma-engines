use std::sync::Arc;

use once_cell::sync::OnceCell;
use psl::datamodel_connector::RelationMode;

use crate::{InternalDataModelWeakRef, Relation, RelationLinkManifestation, RelationRef};

#[derive(Debug)]
pub struct RelationBuilder {
    pub name: String,
    pub manifestation: RelationLinkManifestation,
    pub model_a_name: String,
    pub model_b_name: String,
    pub relation_mode: RelationMode,
}

impl RelationBuilder {
    pub fn build(self, internal_data_model: InternalDataModelWeakRef) -> RelationRef {
        let relation = Relation {
            name: self.name,
            manifestation: self.manifestation,
            model_a_name: self.model_a_name,
            model_b_name: self.model_b_name,
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
