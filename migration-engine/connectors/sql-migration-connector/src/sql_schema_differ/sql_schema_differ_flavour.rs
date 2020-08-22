use super::{ColumnDiffer, SqlSchemaDiffer};
use crate::sql_migration::AlterEnum;
use std::collections::HashSet;

mod mysql;
mod postgres;
mod sqlite;

/// Trait to specialize SQL schema diffing (resulting in migration steps) by SQL backend.
pub(crate) trait SqlSchemaDifferFlavour {
    fn alter_enums(&self, _differ: &SqlSchemaDiffer<'_>) -> Vec<AlterEnum> {
        Vec::new()
    }

    fn column_type_changed(&self, differ: &ColumnDiffer<'_>) -> bool {
        differ.previous.column_type_family() != differ.next.column_type_family()
    }

    /// Return the tables that cannot be migrated without being redefined. This is currently useful only on SQLite.
    fn tables_to_redefine(&self, _differ: &SqlSchemaDiffer<'_>) -> HashSet<String> {
        HashSet::new()
    }

    /// By implementing this method, the flavour signals the differ that specific tables should be ignored. This is mostly for system tables.
    fn table_should_be_ignored(&self, _table_name: &str) -> bool {
        false
    }
}
