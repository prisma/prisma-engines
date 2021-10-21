use std::collections::{HashMap, HashSet};

use crate::{
    ast::{FieldId, ModelId},
    transform::ast_to_dml::db::{walkers::RelationName, ParserDatabase},
};

type RelationIdentifier<'ast> = (ModelId, ModelId, RelationName<'ast>);

#[derive(Clone, Copy)]
pub(super) enum NameTaken {
    Index,
    Unique,
    PrimaryKey,
}

pub(super) struct Names<'ast> {
    pub(super) relation_names: HashMap<RelationIdentifier<'ast>, Vec<FieldId>>,
    index_names: HashMap<ModelId, HashSet<&'ast str>>,
    unique_names: HashMap<ModelId, HashSet<&'ast str>>,
    primary_key_names: HashMap<ModelId, &'ast str>,
}

impl<'ast> Names<'ast> {
    pub(super) fn new(db: &ParserDatabase<'ast>) -> Self {
        let mut relation_names: HashMap<RelationIdentifier<'ast>, Vec<FieldId>> = HashMap::new();
        let mut index_names: HashMap<ModelId, HashSet<&'ast str>> = HashMap::new();
        let mut unique_names: HashMap<ModelId, HashSet<&'ast str>> = HashMap::new();
        let mut primary_key_names: HashMap<ModelId, &'ast str> = HashMap::new();

        for model in db.walk_models() {
            for field in model.relation_fields() {
                let model_id = field.model().model_id();
                let related_model_id = field.related_model().model_id();

                let identifier = (model_id, related_model_id, field.relation_name());
                let field_ids = relation_names.entry(identifier).or_default();

                field_ids.push(field.field_id());
            }

            for index in model.indexes() {
                if let Some(name) = index.attribute().name {
                    if index.is_unique() {
                        unique_names.entry(index.model_id).or_default().insert(name);
                    } else {
                        index_names.entry(index.model_id).or_default().insert(name);
                    }
                }
            }

            if let Some(pk) = model.primary_key().and_then(|pk| pk.name()) {
                primary_key_names.insert(model.model_id(), pk);
            }
        }

        Self {
            relation_names,
            index_names,
            unique_names,
            primary_key_names,
        }
    }

    pub(super) fn name_taken(&self, model_id: ModelId, name: &str) -> Vec<NameTaken> {
        let mut result = Vec::new();

        if self
            .index_names
            .get(&model_id)
            .map(|names| names.contains(name))
            .unwrap_or(false)
        {
            result.push(NameTaken::Index);
        }

        if self
            .unique_names
            .get(&model_id)
            .map(|names| names.contains(name))
            .unwrap_or(false)
        {
            result.push(NameTaken::Unique);
        }

        if self
            .primary_key_names
            .get(&model_id)
            .map(|pk| *pk == name)
            .unwrap_or(false)
        {
            result.push(NameTaken::PrimaryKey);
        }

        result
    }
}
