use datamodel_connector::Connector;
use parser_database::walkers::IndexName;

use super::constraint_namespace::ConstraintNamespace;
use std::collections::{HashMap, HashSet};

use crate::{
    ast::{FieldId, ModelId},
    transform::ast_to_dml::db::{walkers::RelationName, ParserDatabase},
};

type RelationIdentifier<'ast> = (ModelId, ModelId, RelationName<'ast>);

#[derive(Clone, Copy)]
pub(super) enum NameTaken {
    ExplicitIndexName,
    GeneratedIndexName,
    ExplicitUniqueName,
    GeneratedUniqueName,
    ExplicitPrimaryKeyName,
    GeneratedPrimaryKeyName,
}

pub(super) struct Names<'ast> {
    pub(super) relation_names: HashMap<RelationIdentifier<'ast>, Vec<FieldId>>,
    index_names: HashMap<ModelId, HashSet<IndexName<'ast>>>,
    unique_names: HashMap<ModelId, HashSet<IndexName<'ast>>>,
    primary_key_names: HashMap<ModelId, IndexName<'ast>>,
    pub(super) constraint_namespace: ConstraintNamespace<'ast>,
}

impl<'ast> Names<'ast> {
    pub(super) fn new(db: &ParserDatabase<'ast>, connector: &dyn Connector) -> Self {
        let mut relation_names: HashMap<RelationIdentifier<'ast>, Vec<FieldId>> = HashMap::new();
        let mut index_names: HashMap<ModelId, HashSet<IndexName<'ast>>> = HashMap::new();
        let mut unique_names: HashMap<ModelId, HashSet<IndexName<'ast>>> = HashMap::new();
        let mut primary_key_names: HashMap<ModelId, IndexName<'ast>> = HashMap::new();

        for model in db.walk_models() {
            for field in model.relation_fields() {
                let model_id = field.model().model_id();
                let related_model_id = field.related_model().model_id();

                let identifier = (model_id, related_model_id, field.relation_name());
                let field_ids = relation_names.entry(identifier).or_default();

                field_ids.push(field.field_id());
            }

            for index in model.indexes() {
                let index_name = index.name();

                if index.is_unique() {
                    unique_names
                        .entry(index.model().model_id())
                        .or_default()
                        .insert(index_name);
                } else {
                    index_names
                        .entry(index.model().model_id())
                        .or_default()
                        .insert(index_name);
                }
            }

            if let Some(pk) = model.primary_key().map(|pk| pk.name()) {
                primary_key_names.insert(model.model_id(), pk);
            }
        }

        Self {
            relation_names,
            index_names,
            unique_names,
            primary_key_names,
            constraint_namespace: infer_namespaces(db, connector),
        }
    }

    pub(super) fn name_taken(&self, model_id: ModelId, name: &str) -> Vec<NameTaken> {
        let mut result = Vec::new();

        if let Some(names) = self.index_names.get(&model_id) {
            if names.get(&IndexName::explicit(name)).is_some() {
                result.push(NameTaken::ExplicitIndexName);
            }

            if names.get(&IndexName::Generated(Some(name.to_string()))).is_some() {
                result.push(NameTaken::GeneratedIndexName);
            }
        }

        if let Some(names) = self.unique_names.get(&model_id) {
            if names.get(&IndexName::explicit(name)).is_some() {
                result.push(NameTaken::ExplicitUniqueName);
            }

            if names.get(&IndexName::Generated(Some(name.to_string()))).is_some() {
                result.push(NameTaken::GeneratedUniqueName);
            }
        }

        if let Some(pk_name) = self
            .primary_key_names
            .get(&model_id)
            .and_then(|pk| if *pk == name { Some(pk) } else { None })
        {
            let pk_taken = match pk_name {
                IndexName::Explicit(_) => NameTaken::ExplicitPrimaryKeyName,
                IndexName::Generated(_) => NameTaken::GeneratedPrimaryKeyName,
            };

            result.push(pk_taken);
        }

        result
    }
}

/// Generate namespaces per database requirements, and add the names to it from the constraints
/// part of the namespace.
fn infer_namespaces<'ast>(db: &ParserDatabase<'ast>, connector: &dyn Connector) -> ConstraintNamespace<'ast> {
    use datamodel_connector::ConstraintScope;

    let mut namespaces = ConstraintNamespace::default();

    for scope in connector.constraint_violation_scopes() {
        match scope {
            ConstraintScope::GlobalKeyIndex => {
                namespaces.add_global_indexes(db, connector, *scope);
            }
            ConstraintScope::GlobalForeignKey => {
                namespaces.add_global_relations(db, connector, *scope);
            }
            ConstraintScope::GlobalPrimaryKeyKeyIndex => {
                namespaces.add_global_primary_keys(db, connector, *scope);
                namespaces.add_global_indexes(db, connector, *scope);
            }
            ConstraintScope::GlobalPrimaryKeyForeignKeyDefault => {
                namespaces.add_global_primary_keys(db, connector, *scope);
                namespaces.add_global_relations(db, connector, *scope);
                namespaces.add_global_default_constraints(db, connector, *scope);
            }
            ConstraintScope::ModelKeyIndex => {
                namespaces.add_local_indexes(db, connector, *scope);
            }
            ConstraintScope::ModelPrimaryKeyKeyIndex => {
                namespaces.add_local_primary_keys(db, connector, *scope);
                namespaces.add_local_indexes(db, connector, *scope);
            }
            ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey => {
                namespaces.add_local_primary_keys(db, connector, *scope);
                namespaces.add_local_indexes(db, connector, *scope);
                namespaces.add_local_relations(db, connector, *scope);
            }
        }
    }

    namespaces.add_local_custom_names_for_primary_keys_and_uniques(db);

    namespaces
}
