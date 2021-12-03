use crate::{ast, transform::ast_to_dml::db::ParserDatabase};
use datamodel_connector::{walker_ext_traits::*, Connector, ConstraintScope};
use std::{borrow::Cow, collections::HashMap, ops::Deref};

/// A constraint namespace consists of two kinds of namespaces:
///
/// - Global ones can be triggering validation errors between different models.
/// - Local ones are only valid in the given model.
#[derive(Debug, Default)]
pub(crate) struct ConstraintNamespace<'ast> {
    global: HashMap<(ConstraintScope, Cow<'ast, str>), usize>,
    local: HashMap<(ast::ModelId, ConstraintScope, Cow<'ast, str>), usize>,
}

impl<'ast> ConstraintNamespace<'ast> {
    /// An iterator of namespace violations with the given name, first globally followed up with
    /// local violations in the given model.
    pub(crate) fn scope_violations(
        &self,
        model_id: ast::ModelId,
        name: ConstraintName<'ast>,
    ) -> impl Iterator<Item = &'ast ConstraintScope> + '_ {
        self.global_scope_violations(name)
            .chain(self.local_scope_violations(model_id, name))
    }

    fn global_scope_violations(&self, name: ConstraintName<'ast>) -> impl Iterator<Item = &'ast ConstraintScope> + '_ {
        name.possible_scopes().filter(
            move |scope| match self.global.get(&(**scope, Cow::from(name.as_ref()))) {
                Some(count) => *count > 1,
                None => false,
            },
        )
    }

    fn local_scope_violations(
        &self,
        model_id: ast::ModelId,
        name: ConstraintName<'ast>,
    ) -> impl Iterator<Item = &'ast ConstraintScope> + '_ {
        name.possible_scopes().filter(move |scope| {
            match self.local.get(&(model_id, **scope, Cow::from(name.as_ref()))) {
                Some(count) => *count > 1,
                None => false,
            }
        })
    }

    /// Add all index and unique constraints from the data model to a global validation scope.
    pub(super) fn add_global_indexes(
        &mut self,
        db: &ParserDatabase<'ast>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for index in db.walk_models().flat_map(|m| m.indexes()) {
            let counter = self
                .global
                .entry((scope, index.final_database_name(connector)))
                .or_default();
            *counter += 1;
        }
    }

    /// Add all foreign key constraints from the data model to a global validation scope.
    pub(super) fn add_global_relations(
        &mut self,
        db: &ParserDatabase<'ast>,
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
        db: &ParserDatabase<'ast>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for model in db.walk_models() {
            if let Some(name) = model.primary_key().and_then(|k| k.final_database_name(connector)) {
                let counter = self.global.entry((scope, name)).or_default();
                *counter += 1;
            }
        }
    }

    /// Add all default constraints from the data model to a global validation scope.
    pub(super) fn add_global_default_constraints(
        &mut self,
        db: &ParserDatabase<'ast>,
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
        db: &ParserDatabase<'ast>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for model in db.walk_models() {
            for index in model.indexes() {
                let counter = self
                    .local
                    .entry((model.model_id(), scope, index.final_database_name(connector)))
                    .or_default();

                *counter += 1;
            }
        }
    }

    /// Add all primary key constraints to separate namespaces per model.
    pub(super) fn add_local_primary_keys(
        &mut self,
        db: &ParserDatabase<'ast>,
        connector: &dyn Connector,
        scope: ConstraintScope,
    ) {
        for model in db.walk_models() {
            if let Some(name) = model.primary_key().and_then(|pk| pk.final_database_name(connector)) {
                let counter = self.local.entry((model.model_id(), scope, name)).or_default();
                *counter += 1;
            }
        }
    }

    /// Add all foreign key constraints to separate namespaces per model.
    pub(super) fn add_local_relations(
        &mut self,
        db: &ParserDatabase<'ast>,
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
pub(crate) enum ConstraintName<'ast> {
    Index(&'ast str),
    Relation(&'ast str),
    Default(&'ast str),
    PrimaryKey(&'ast str),
}

impl<'ast> ConstraintName<'ast> {
    /// An iterator of scopes the given name should be checked against.
    fn possible_scopes(self) -> impl Iterator<Item = &'ast ConstraintScope> {
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

impl<'ast> AsRef<str> for ConstraintName<'ast> {
    fn as_ref(&self) -> &str {
        match self {
            ConstraintName::Index(x) => x,
            ConstraintName::Relation(x) => x,
            ConstraintName::Default(x) => x,
            ConstraintName::PrimaryKey(x) => x,
        }
    }
}

impl<'ast> Deref for ConstraintName<'ast> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
