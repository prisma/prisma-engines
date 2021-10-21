use crate::prelude::*;
use datamodel::{ReferentialAction, RelationInfo};
use once_cell::sync::OnceCell;
use std::{fmt::Debug, sync::Arc};

#[derive(Debug)]
pub struct RelationFieldBuilder {
    pub name: String,
    pub is_required: bool,
    pub is_list: bool,
    pub relation_name: String,
    pub relation_side: RelationSide,
    pub relation_info: RelationInfo,
    pub on_delete_default: ReferentialAction,
    pub on_update_default: ReferentialAction,
}

impl RelationFieldBuilder {
    pub fn build(self, model: ModelWeakRef) -> RelationFieldRef {
        Arc::new(RelationField {
            name: self.name,
            is_required: self.is_required,
            is_list: self.is_list,
            relation_name: self.relation_name,
            relation_side: self.relation_side,
            model,
            relation: OnceCell::new(),
            relation_info: self.relation_info,
            fields: OnceCell::new(),
            on_delete_default: self.on_delete_default,
            on_update_default: self.on_update_default,
        })
    }
}
