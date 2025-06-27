use crate::{prelude::*, CompositeType, InternalEnum};
use psl::parser_database as db;
use std::sync::Arc;

pub(crate) type InternalDataModelRef = InternalDataModel;

#[derive(Debug, Clone)]
pub struct InternalDataModel {
    pub schema: Arc<psl::ValidatedSchema>,
}

impl InternalDataModel {
    pub fn models(&self) -> impl Iterator<Item = Model> + '_ {
        self.schema
            .db
            .walk_models()
            .chain(self.schema.db.walk_views())
            .filter(|model| !model.is_ignored())
            .map(|model| self.clone().zip(model.id))
    }

    pub fn composite_types(&self) -> impl Iterator<Item = CompositeType> + '_ {
        self.schema
            .db
            .walk_composite_types()
            .map(move |ct| self.clone().zip(ct.id))
    }

    pub fn relations(&self) -> impl Iterator<Item = Relation> + Clone + '_ {
        self.schema
            .db
            .walk_relations()
            .filter(|relation| !relation.is_ignored())
            .map(|relation| self.clone().zip(relation.id))
    }

    pub fn find_enum(&self, name: &str) -> crate::Result<InternalEnum> {
        self.schema
            .db
            .find_enum(name)
            .map(|enum_walker| self.clone().zip(enum_walker.id))
            .ok_or_else(|| DomainError::EnumNotFound { name: name.to_string() })
    }

    pub fn find_model(&self, name: &str) -> crate::Result<Model> {
        self.schema
            .db
            .walk_models()
            .chain(self.schema.db.walk_views())
            .find(|model| model.name() == name)
            .map(|m| self.clone().zip(m.id))
            .ok_or_else(|| DomainError::ModelNotFound { name: name.to_string() })
    }

    pub fn find_composite_type_by_id(&self, ctid: db::CompositeTypeId) -> CompositeType {
        self.clone().zip(ctid)
    }

    pub fn find_model_by_id(&self, model_id: db::ModelId) -> Model {
        self.clone().zip(model_id)
    }

    /// Finds all inline relation fields pointing to the given model.
    pub fn fields_pointing_to_model(&self, model: &Model) -> impl Iterator<Item = RelationFieldRef> + '_ {
        self.walk(model.id)
            .relations_to()
            .filter_map(|rel| rel.refine().as_inline())
            .filter_map(|inline_rel| inline_rel.forward_relation_field())
            .map(move |rf| self.clone().zip(rf.id))
    }

    pub fn walk<I>(&self, id: I) -> psl::parser_database::walkers::Walker<'_, I> {
        self.schema.db.walk(id)
    }

    pub fn zip<I>(self, id: I) -> crate::Zipper<I> {
        crate::Zipper { id, dm: self }
    }
}
