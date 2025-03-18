use super::SqlSchemaDifferFlavour;
use crate::{
    migration_pair::MigrationPair, sql_schema_differ::column::ColumnTypeChange,
    sql_schema_differ::differ_database::DifferDatabase,
};
use sql_schema_describer::{walkers::TableColumnWalker, ColumnTypeFamily};

#[derive(Debug, Default)]
pub struct SqliteSchemaDifferFlavour;

impl SqlSchemaDifferFlavour for SqliteSchemaDifferFlavour {
    fn can_rename_foreign_key(&self) -> bool {
        false
    }

    fn can_redefine_tables_with_inbound_foreign_keys(&self) -> bool {
        true
    }

    fn can_rename_index(&self) -> bool {
        false
    }

    fn column_autoincrement_changed(&self, _columns: MigrationPair<TableColumnWalker<'_>>) -> bool {
        false
    }

    fn column_type_change(&self, differ: MigrationPair<TableColumnWalker<'_>>) -> Option<ColumnTypeChange> {
        match (differ.previous.column_type_family(), differ.next.column_type_family()) {
            (a, b) if a == b => None,
            (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
            (_, _) => Some(ColumnTypeChange::RiskyCast),
        }
    }

    fn should_drop_indexes_from_dropped_tables(&self) -> bool {
        true
    }

    fn set_tables_to_redefine(&self, differ: &mut DifferDatabase<'_>) {
        differ.tables_to_redefine = differ
            .table_pairs()
            .filter(|differ| {
                differ.created_primary_key().is_some()
                    || differ.dropped_primary_key().is_some()
                    || differ.primary_key_changed()
                    || differ.dropped_columns().next().is_some()
                    || differ.added_columns().any(|col| col.arity().is_required())
                    || differ.any_column_changed()
                    || differ.created_foreign_keys().next().is_some()
                    || differ.dropped_foreign_keys().next().is_some()
            })
            .map(|table| table.table_ids())
            .collect();
    }

    fn should_drop_foreign_keys_from_dropped_tables(&self) -> bool {
        false
    }

    fn should_push_foreign_keys_from_created_tables(&self) -> bool {
        false
    }

    fn has_unnamed_foreign_keys(&self) -> bool {
        true
    }
}
