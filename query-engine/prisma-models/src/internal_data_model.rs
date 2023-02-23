use crate::{prelude::*, CompositeTypeRef, InternalEnumRef};
use once_cell::sync::OnceCell;
use psl::schema_ast::ast;
use std::sync::{Arc, Weak};

pub type InternalDataModelRef = Arc<InternalDataModel>;
pub type InternalDataModelWeakRef = Weak<InternalDataModel>;

#[derive(Debug)]
pub struct InternalDataModel {
    pub(crate) models: OnceCell<Vec<ModelRef>>,

    pub schema: Arc<psl::ValidatedSchema>,
}

impl InternalDataModel {
    pub fn models(&self) -> &[ModelRef] {
        self.models.get().unwrap()
    }

    pub fn composite_types<'a>(self: &'a Arc<Self>) -> impl Iterator<Item = CompositeTypeRef> + 'a {
        self.schema
            .db
            .walk_composite_types()
            .map(move |ct| self.clone().zip(ct.id))
    }

    pub fn models_cloned(&self) -> Vec<ModelRef> {
        self.models.get().unwrap().iter().map(Arc::clone).collect()
    }

    pub fn relations<'a>(self: &'a Arc<Self>) -> impl Iterator<Item = RelationRef> + Clone + 'a {
        self.schema
            .db
            .walk_relations()
            .filter(|relation| !relation.is_ignored())
            .map(|relation| self.clone().zip(relation.id))
    }

    pub fn find_enum(self: &Arc<Self>, name: &str) -> crate::Result<InternalEnumRef> {
        self.schema
            .db
            .find_enum(name)
            .map(|enum_walker| self.clone().zip(enum_walker.id))
            .ok_or_else(|| DomainError::EnumNotFound { name: name.to_string() })
    }

    pub fn find_model(&self, name: &str) -> crate::Result<ModelRef> {
        self.models
            .get()
            .and_then(|models| models.iter().find(|model| model.name == name))
            .cloned()
            .ok_or_else(|| DomainError::ModelNotFound { name: name.to_string() })
    }

    pub fn find_composite_type_by_id(self: &Arc<Self>, ctid: ast::CompositeTypeId) -> CompositeTypeRef {
        self.clone().zip(ctid)
    }

    pub fn find_model_by_id(&self, model_id: ast::ModelId) -> ModelRef {
        self.models
            .get()
            .and_then(|models| models.iter().find(|model| model.id == model_id))
            .cloned()
            .unwrap()
    }

    /// Finds all inline relation fields pointing to the given model.
    pub fn fields_pointing_to_model(self: &Arc<Self>, model: &ModelRef) -> Vec<RelationFieldRef> {
        self.walk(model.id)
            .relations_to()
            .filter_map(|rel| rel.refine().as_inline())
            .filter_map(|inline_rel| inline_rel.forward_relation_field())
            .map(move |rf| self.clone().zip(rf.id))
            .collect()
    }

    pub fn walk<I>(&self, id: I) -> psl::parser_database::walkers::Walker<I> {
        self.schema.db.walk(id)
    }

    pub fn zip<I>(self: Arc<InternalDataModel>, id: I) -> crate::Zipper<I> {
        crate::Zipper { id, dm: self }
    }
}
