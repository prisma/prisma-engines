use crate::{prelude::*, CompositeTypeRef, InternalEnumRef};
use psl::schema_ast::ast;
use std::sync::{Arc, Weak};

pub type InternalDataModelRef = Arc<InternalDataModel>;
pub type InternalDataModelWeakRef = Weak<InternalDataModel>;

#[derive(Debug)]
pub struct InternalDataModel {
    pub schema: Arc<psl::ValidatedSchema>,
}

impl InternalDataModel {
    pub fn models<'a>(self: &'a Arc<Self>) -> impl Iterator<Item = ModelRef> + 'a {
        self.schema
            .db
            .walk_models()
            .chain(self.schema.db.walk_views())
            .filter(|model| !model.is_ignored())
            .map(|model| self.clone().zip(model.id))
    }

    pub fn composite_types<'a>(self: &'a Arc<Self>) -> impl Iterator<Item = CompositeTypeRef> + 'a {
        self.schema
            .db
            .walk_composite_types()
            .map(move |ct| self.clone().zip(ct.id))
    }

    pub fn models_cloned(self: &Arc<Self>) -> Vec<ModelRef> {
        self.models().collect()
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

    pub fn find_model(self: &Arc<Self>, name: &str) -> crate::Result<ModelRef> {
        self.schema
            .db
            .walk_models()
            .chain(self.schema.db.walk_views())
            .find(|model| model.name() == name)
            .map(|m| self.clone().zip(m.id))
            .ok_or_else(|| DomainError::ModelNotFound { name: name.to_string() })
    }

    pub fn find_composite_type_by_id(self: &Arc<Self>, ctid: ast::CompositeTypeId) -> CompositeTypeRef {
        self.clone().zip(ctid)
    }

    pub fn find_model_by_id(self: &Arc<Self>, model_id: ast::ModelId) -> ModelRef {
        self.clone().zip(model_id)
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
