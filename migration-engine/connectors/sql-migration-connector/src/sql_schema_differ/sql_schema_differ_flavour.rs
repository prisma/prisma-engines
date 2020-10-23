use super::{column::ColumnDiffer, ColumnTypeChange, SqlSchemaDiffer};
use crate::sql_migration::AlterEnum;
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

    /// Return whether a column's type needs to be migrated, and how.
    fn column_type_change(&self, differ: &ColumnDiffer<'_>) -> Option<ColumnTypeChange> {
        if differ.previous.column_type_family() != differ.next.column_type_family() {
            Some(ColumnTypeChange::RiskyCast)
        } else {
            None
        }
    }

    /// Return whether an index should be renamed by the migration.
    fn index_should_be_renamed(&self, previous: &IndexWalker<'_>, next: &IndexWalker<'_>) -> bool {
        previous.name() != next.name()
    }

    /// Whether `AddForeignKey` steps should be generated for created tables.
    fn should_push_foreign_keys_from_created_tables(&self) -> bool {
        true
    }

    /// Return the tables that cannot be migrated without being redefined. This is currently useful only on SQLite.
    fn tables_to_redefine(&self, _differ: &SqlSchemaDiffer<'_>) -> HashSet<String> {
        HashSet::new()
    }

    /// By implementing this method, the flavour signals the differ that
    /// specific tables should be ignored. This is mostly for system tables.
    fn table_should_be_ignored(&self, _table_name: &str) -> bool {
        false
    }
}
