use super::{column::ColumnDiffer, ColumnTypeChange, SqlSchemaDiffer};
use crate::{pair::Pair, sql_migration::AlterEnum};
use sql_schema_describer::walkers::IndexWalker;
use std::collections::HashSet;

mod mssql;
mod mysql;
mod postgres;
mod sqlite;

/// Trait to specialize SQL schema diffing (resulting in migration steps) by SQL backend.
pub(crate) trait SqlSchemaDifferFlavour {
    /// Return potential `AlterEnum` steps.
    fn alter_enums(&self, _differ: &SqlSchemaDiffer<'_>) -> Vec<AlterEnum> {
        Vec::new()
    }

    /// If this returns `true`, the differ will generate
    /// SqlMigrationStep::RedefineIndex steps instead of
    /// SqlMigrationStep::AlterIndex.
    fn can_alter_index(&self) -> bool {
        true
    }

    /// Return whether a column's type needs to be migrated, and how.
    fn column_type_change(&self, differ: &ColumnDiffer<'_>) -> Option<ColumnTypeChange> {
        if differ.previous.column_type_family() != differ.next.column_type_family() {
            Some(ColumnTypeChange::RiskyCast)
        } else {
            None
        }
    }

    /// Return whether an index should be renamed by the migration.
    fn index_should_be_renamed(&self, indexes: &Pair<IndexWalker<'_>>) -> bool {
        indexes.previous().name() != indexes.next().name()
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
}
