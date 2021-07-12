use super::{ColumnTypeChange, SqlSchemaDiffer};
use crate::{pair::Pair, sql_migration::SqlMigrationStep, sql_schema_differ};
use sql_schema_describer::{
    walkers::{ColumnWalker, IndexWalker},
    ColumnId,
};
use std::collections::HashSet;

mod mssql;
mod mysql;
mod postgres;
mod sqlite;

/// Trait to specialize SQL schema diffing (resulting in migration steps) by SQL backend.
pub(crate) trait SqlSchemaDifferFlavour {
    /// If this returns `true`, the differ will generate
    /// SqlMigrationStep::RedefineIndex steps instead of
    /// SqlMigrationStep::AlterIndex.
    fn can_alter_index(&self) -> bool {
        true
    }

    /// Returns true only if the database can cope with an optional column
    /// constrained by a foreign key being made NOT NULL.
    fn can_cope_with_foreign_key_column_becoming_nonnullable(&self) -> bool {
        true
    }

    /// Return whether a column's type needs to be migrated, and how.
    fn column_type_change(&self, differ: Pair<ColumnWalker<'_>>) -> Option<ColumnTypeChange> {
        if differ.previous.column_type_family() != differ.next.column_type_family() {
            Some(ColumnTypeChange::RiskyCast)
        } else {
            None
        }
    }

    /// Push enum-related steps.
    fn push_enum_steps(&self, _differ: &SqlSchemaDiffer<'_>, _steps: &mut Vec<SqlMigrationStep>) {}

    /// Returns whether the underlying database implicitly drops indexes on dropped (and potentially recreated) columns.
    fn indexes_should_be_recreated_after_column_drop(&self) -> bool {
        false
    }

    /// Return whether an index should be renamed by the migration.
    fn index_should_be_renamed(&self, indexes: &Pair<IndexWalker<'_>>) -> bool {
        indexes.previous().name() != indexes.next().name()
    }

    fn lower_cases_table_names(&self) -> bool {
        false
    }

    /// Evaluate indexes/constraints that need to be dropped and re-created based on other changes in the schema
    fn push_index_changes_for_column_changes(
        &self,
        _table: &sql_schema_differ::TableDiffer<'_, '_>,
        _column_index: Pair<ColumnId>,
        _column_changes: sql_schema_differ::ColumnChanges,
        _steps: &mut Vec<SqlMigrationStep>,
    ) {
    }

    /// Whether the differ should produce CreateIndex steps for the indexes of
    /// new tables.
    fn should_create_indexes_from_created_tables(&self) -> bool {
        true
    }

    /// Whether the indexes of dropped tables should be dropped before the table
    /// is dropped.
    fn should_drop_indexes_from_dropped_tables(&self) -> bool {
        false
    }

    /// Whether the foreign keys of dropped tables should be dropped before the table
    /// is dropped.
    fn should_drop_foreign_keys_from_dropped_tables(&self) -> bool {
        true
    }

    /// Whether to skip diffing JSON defaults.
    fn should_ignore_json_defaults(&self) -> bool {
        false
    }

    /// Whether `AddForeignKey` steps should be generated for created tables.
    fn should_push_foreign_keys_from_created_tables(&self) -> bool {
        true
    }

    /// Whether indexes matching a foreign key should be skipped.
    fn should_skip_fk_indexes(&self) -> bool {
        false
    }

    /// Whether a specific index should *not* be produced.
    fn should_skip_index_for_new_table(&self, _index: &IndexWalker<'_>) -> bool {
        false
    }

    /// Whether the primary key should be recreated if the column part of it is
    /// recreated.
    fn should_recreate_the_primary_key_on_column_recreate(&self) -> bool {
        false
    }

    fn table_names_match(&self, names: Pair<&str>) -> bool {
        names.previous() == names.next()
    }

    /// Return the tables that cannot be migrated without being redefined. This
    /// is currently useful only on SQLite.
    fn tables_to_redefine(&self, _differ: &SqlSchemaDiffer<'_>) -> HashSet<String> {
        HashSet::new()
    }

    /// By implementing this method, the flavour signals the differ that
    /// specific tables should be ignored. This is mostly for system tables.
    fn table_should_be_ignored(&self, _table_name: &str) -> bool {
        false
    }

    fn view_should_be_ignored(&self, _view_name: &str) -> bool {
        false
    }
}
