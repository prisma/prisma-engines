use crate::prelude::*;
use dml::{FieldArity, ReferentialAction, RelationInfo};
use once_cell::sync::OnceCell;
use psl::parser_database::walkers::RelationFieldId;
use std::{fmt::Debug, sync::Arc};

#[derive(Debug)]
pub struct RelationFieldBuilder {
    pub id: RelationFieldId,
    pub name: String,
    pub arity: FieldArity,
    pub relation_name: String,
    pub relation_side: RelationSide,
    pub relation_info: RelationInfo,
    pub on_delete_default: ReferentialAction,
    pub on_update_default: ReferentialAction,
}

impl RelationFieldBuilder {
    pub fn build(self, model: ModelWeakRef) -> RelationFieldRef {
        Arc::new(RelationField {
            id: self.id,
            name: self.name,
            arity: self.arity,
            relation_name: self.relation_name,
            relation_side: self.relation_side,
            model,
            relation_info: self.relation_info,
            fields: OnceCell::new(),
            on_delete_default: self.on_delete_default,
            on_update_default: self.on_update_default,
        })
    }
}
