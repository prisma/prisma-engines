use crate::{ast, transform::ast_to_dml::db::ParserDatabase};
use datamodel_connector::{walker_ext_traits::*, Connector, ConstraintScope};
use std::{borrow::Cow, collections::HashMap, ops::Deref};

/// A constraint namespace consists of two kinds of namespaces:
///
/// - Global ones can be triggering validation errors between different models.
/// - Local ones are only valid in the given model.
#[derive(Debug, Default)]
pub(crate) struct ConstraintNamespace<'db> {
    global: HashMap<(ConstraintScope, Cow<'db, str>), usize>,
    local: HashMap<(ast::ModelId, ConstraintScope, Cow<'db, str>), usize>,
    local_custom_name: HashMap<(ast::ModelId, Cow<'db, str>), usize>,
}

impl<'db> ConstraintNamespace<'db> {
    /// An iterator of namespace violations with the given name, first globally followed up with
    /// local violations in the given model.
    pub(crate) fn constraint_name_scope_violations(
        &self,
        model_id: ast::ModelId,
        name: ConstraintName<'db>,
    ) -> impl Iterator<Item = &'db ConstraintScope> + '_ {
        self.global_constraint_name_scope_violations(name)
            .chain(self.local_constraint_name_scope_violations(model_id, name))
    }

    fn global_constraint_name_scope_violations(
        &self,
        name: ConstraintName<'db>,
    ) -> impl Iterator<Item = &'db ConstraintScope> + '_ {
        name.possible_scopes().filter(
            move |scope| match self.global.get(&(**scope, Cow::from(name.as_ref()))) {
                Some(count) => *count > 1,
                None => false,
            },
        )
    }

    fn local_constraint_name_scope_violations(
        &self,
        model_id: ast::ModelId,
        name: ConstraintName<'db>,
    ) -> impl Iterator<Item = &'db ConstraintScope> + '_ {
        name.possible_scopes().filter(move |scope| {
            match self.local.get(&(model_id, **scope, Cow::from(name.as_ref()))) {
                Some(count) => *count > 1,
                None => false,
            }
        })
    }

    pub(crate) fn local_custom_name_scope_violations(&self, model_id: ast::ModelId, name: &'db str) -> bool {
        match self.local_custom_name.get(&(model_id, Cow::from(name))) {
            Some(count) => *count > 1,
            None => false,
        }
    }

    /// Add all index and unique constraints from the data model to a global validation scope.
    pub(super) fn add_global_indexes(
        &mut self,
        db: &ParserDatabase<'db>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for index in db.walk_models().flat_map(|m| m.indexes()) {
            let counter = self
                .global
                .entry((scope, index.constraint_name(connector)))
                .or_default();
            *counter += 1;
        }
    }

    /// Add all foreign key constraints from the data model to a global validation scope.
    pub(super) fn add_global_relations(
        &mut self,
        db: &'db ParserDatabase<'_>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for name in db
            .walk_relations()
            .filter_map(|r| r.refine().as_inline())
            .map(|inline_relation| inline_relation.constraint_name(connector))
        {
            let counter = self.global.entry((scope, name)).or_default();
            *counter += 1;
        }
    }

    /// Add all primary key constraints from the data model to a global validation scope.
    pub(super) fn add_global_primary_keys(
        &mut self,
        db: &ParserDatabase<'db>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for model in db.walk_models() {
            if let Some(name) = model.primary_key().and_then(|k| k.constraint_name(connector)) {
                let counter = self.global.entry((scope, name)).or_default();
                *counter += 1;
            }
        }
    }

    /// Add all default constraints from the data model to a global validation scope.
    pub(super) fn add_global_default_constraints(
        &mut self,
        db: &ParserDatabase<'db>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for field in db.walk_models().flat_map(|m| m.scalar_fields()) {
            if let Some(name) = field.default_value().map(|d| d.constraint_name(connector)) {
                let name = match name {
                    Cow::Borrowed(bor) => Cow::Owned(bor.to_string()),
                    Cow::Owned(own) => Cow::Owned(own),
                };

                let counter = self.global.entry((scope, name)).or_default();
                *counter += 1;
            }
        }
    }

    /// Add all index and unique constraints to separate namespaces per model.
    pub(super) fn add_local_indexes(
        &mut self,
        db: &'db ParserDatabase<'_>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for model in db.walk_models() {
            for index in model.indexes() {
                let counter = self
                    .local
                    .entry((model.model_id(), scope, index.constraint_name(connector)))
                    .or_default();

                *counter += 1;
            }
        }
    }

    /// Add all primary key constraints to separate namespaces per model.
    pub(super) fn add_local_primary_keys(
        &mut self,
        db: &ParserDatabase<'db>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for model in db.walk_models() {
            if let Some(name) = model.primary_key().and_then(|pk| pk.constraint_name(connector)) {
                let counter = self.local.entry((model.model_id(), scope, name)).or_default();
                *counter += 1;
            }
        }
    }

    /// Add all primary key and unique index custom names to separate namespaces per model.
    pub(super) fn add_local_custom_names_for_primary_keys_and_uniques(&mut self, db: &ParserDatabase<'db>) {
        for model in db.walk_models() {
            if let Some(name) = model.primary_key().and_then(|pk| pk.name()) {
                let counter = self
                    .local_custom_name
                    .entry((model.model_id(), Cow::from(name)))
                    .or_default();
                *counter += 1;
            }
            for index in model.indexes() {
                if let Some(name) = index.name() {
                    let counter = self
                        .local_custom_name
                        .entry((model.model_id(), Cow::from(name)))
                        .or_default();
                    *counter += 1;
                }
            }
        }
    }

    /// Add all foreign key constraints to separate namespaces per model.
    pub(super) fn add_local_relations(
        &mut self,
        db: &'db ParserDatabase<'_>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for model in db.walk_models() {
            for name in model
                .relations_from()
                .filter_map(|r| r.refine().as_inline())
                .map(|r| r.constraint_name(connector))
            {
                let counter = self.local.entry((model.model_id(), scope, name)).or_default();

                *counter += 1;
            }
        }
    }
}

/// A constraint name marked by the constraint type it belongs. The variant decides on which
/// validation scopes it will be checked on.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ConstraintName<'db> {
    Index(&'db str),
    Relation(&'db str),
    Default(&'db str),
    PrimaryKey(&'db str),
}

impl<'db> ConstraintName<'db> {
    /// An iterator of scopes the given name should be checked against.
    fn possible_scopes(self) -> impl Iterator<Item = &'db ConstraintScope> {
        use ConstraintScope::*;

        match self {
            ConstraintName::Index(_) => [
                GlobalKeyIndex,
                GlobalPrimaryKeyKeyIndex,
                ModelKeyIndex,
                ModelPrimaryKeyKeyIndex,
            ]
            .iter(),
            ConstraintName::Relation(_) => [
                GlobalForeignKey,
                GlobalPrimaryKeyForeignKeyDefault,
                ModelPrimaryKeyKeyIndexForeignKey,
            ]
            .iter(),
            ConstraintName::Default(_) => [GlobalPrimaryKeyForeignKeyDefault].iter(),
            ConstraintName::PrimaryKey(_) => [
                GlobalPrimaryKeyForeignKeyDefault,
                GlobalPrimaryKeyKeyIndex,
                ModelPrimaryKeyKeyIndex,
                ModelPrimaryKeyKeyIndexForeignKey,
            ]
            .iter(),
        }
    }
}

impl<'db> AsRef<str> for ConstraintName<'db> {
    fn as_ref(&self) -> &str {
        match self {
            ConstraintName::Index(x) => x,
            ConstraintName::Relation(x) => x,
            ConstraintName::Default(x) => x,
            ConstraintName::PrimaryKey(x) => x,
        }
    }
}

impl<'db> Deref for ConstraintName<'db> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
