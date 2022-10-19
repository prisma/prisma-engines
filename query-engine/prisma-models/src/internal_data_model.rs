use crate::{parent_container::ParentContainer, prelude::*, CompositeTypeRef, InternalEnumRef};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Weak};

pub type InternalDataModelRef = Arc<InternalDataModel>;
pub type InternalDataModelWeakRef = Weak<InternalDataModel>;

#[derive(Debug)]
pub struct InternalDataModel {
    pub(crate) models: OnceCell<Vec<ModelRef>>,
    pub(crate) composite_types: OnceCell<Vec<CompositeTypeRef>>,
    pub(crate) relations: OnceCell<Vec<RelationRef>>,
    pub(crate) relation_fields: OnceCell<Vec<RelationFieldRef>>,

    /// Todo clarify / rename.
    /// The db name influences how data is queried from the database.
    /// E.g. this influences the schema part of a postgres query: `database`.`schema`.`table`.
    /// Other connectors do not use `schema`, like postgres does, and this variable would
    /// influence the `database` part instead.
    pub db_name: String,
    pub enums: Vec<InternalEnumRef>,
}

impl InternalDataModel {
    pub(crate) fn finalize(&self) {
        self.models().iter().for_each(|model| model.finalize());
    }

    pub fn models(&self) -> &[ModelRef] {
        self.models.get().unwrap()
    }

    pub fn composite_types(&self) -> &[CompositeTypeRef] {
        self.composite_types.get().unwrap()
    }

    pub fn models_cloned(&self) -> Vec<ModelRef> {
        self.models.get().unwrap().iter().map(Arc::clone).collect()
    }

    pub fn relations(&self) -> &[RelationRef] {
        self.relations.get().unwrap().as_slice()
    }

    pub fn find_enum(&self, name: &str) -> crate::Result<InternalEnumRef> {
        self.enums
            .iter()
            .find(|e| e.name == name)
            .cloned()
            .ok_or_else(|| DomainError::EnumNotFound { name: name.to_string() })
    }

    pub fn find_model(&self, name: &str) -> crate::Result<ModelRef> {
        self.models
            .get()
            .and_then(|models| models.iter().find(|model| model.name == name))
            .cloned()
            .ok_or_else(|| DomainError::ModelNotFound { name: name.to_string() })
    }

    /// This method takes the two models at the ends of the relation as a first argument, because
    /// relation names are scoped by the pair of models in the relation. Relation names are _not_
    /// globally unique.
    pub fn find_relation(&self, model_names: (&str, &str), relation_name: &str) -> crate::Result<RelationWeakRef> {
        self.relations
            .get()
            .and_then(|relations| {
                relations
                    .iter()
                    .find(|relation| relation_matches(relation, model_names, relation_name))
            })
            .map(Arc::downgrade)
            .ok_or_else(|| DomainError::RelationNotFound {
                name: relation_name.to_owned(),
            })
    }

    /// Finds all non-list relation fields pointing to the given model.
    /// `required` may narrow down the returned fields to required fields only. Returns all on `false`.
    pub fn fields_pointing_to_model(&self, model: &ModelRef, required: bool) -> Vec<RelationFieldRef> {
        self.relation_fields()
            .iter()
            .filter(|rf| &rf.related_model() == model) // All relation fields pointing to `model`.
            .filter(|rf| rf.is_inlined_on_enclosing_model()) // Not a list, not a virtual field.
            .filter(|rf| !required || rf.is_required()) // If only required fields should be returned
            .map(Arc::clone)
            .collect()
    }

    /// Finds all relation fields where the foreign key refers to the given field (as either singular or compound).
    pub fn fields_refering_to_field(&self, field: &ScalarFieldRef) -> Vec<RelationFieldRef> {
        match &field.container {
            ParentContainer::Model(model) => {
                let model_name = &model.upgrade().unwrap().name;

                self.relation_fields()
                    .iter()
                    .filter(|rf| &rf.relation_info.referenced_model == model_name)
                    .filter(|rf| rf.relation_info.references.contains(&field.name))
                    .map(Arc::clone)
                    .collect()
            }
            // Relation fields can not refer to composite fields.
            ParentContainer::CompositeType(_) => vec![],
        }
    }

    pub fn relation_fields(&self) -> &[RelationFieldRef] {
        self.relation_fields
            .get_or_init(|| {
                self.models()
                    .iter()
                    .flat_map(|model| model.fields().relation())
                    .collect()
            })
            .as_slice()
    }
}

/// A relation's "primary key" in a Prisma schema is the relation name (defaulting to an empty
/// string) qualified by the one or two models involved in the relation.
///
/// In other words, the scope for a relation name is only between two models. Every pair of models
/// has its own scope for relation names.
fn relation_matches(relation: &Relation, model_names: (&str, &str), relation_name: &str) -> bool {
    if relation.name != relation_name {
        return false;
    }

    if relation.is_self_relation() && model_names.0 == model_names.1 && relation.model_a_name == model_names.0 {
        return true;
    }

    if model_names.0 == relation.model_a_name && model_names.1 == relation.model_b_name {
        return true;
    }

    if model_names.0 == relation.model_b_name && model_names.1 == relation.model_a_name {
        return true;
    }

    false
}
