use sql_schema_describer::ColumnTypeFamily;

use super::SqlSchemaDifferFlavour;
use crate::{
    flavour::SqliteFlavour,
    sql_schema_differ::column::{ColumnDiffer, ColumnTypeChange},
    sql_schema_differ::SqlSchemaDiffer,
};
use std::collections::HashSet;

impl SqlSchemaDifferFlavour for SqliteFlavour {
    fn column_type_change(&self, differ: &ColumnDiffer<'_>) -> Option<ColumnTypeChange> {
        match (differ.previous.column_type_family(), differ.next.column_type_family()) {
            (a, b) if a == b => None,
            (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
            (_, _) => Some(ColumnTypeChange::RiskyCast),
        }
    }

    fn should_drop_indexes_from_dropped_tables(&self) -> bool {
        true
    }

    fn tables_to_redefine(&self, differ: &SqlSchemaDiffer<'_>) -> HashSet<String> {
        differ
            .table_pairs()
            .filter(|differ| {
                differ.created_primary_key().is_some()
                    || differ.dropped_primary_key().is_some()
                    || differ.dropped_columns().next().is_some()
                    || differ.added_columns().any(|col| col.arity().is_required())
                    || differ.column_pairs().any(|columns| columns.all_changes().0.differs_in_something())
                    // ALTER INDEX does not exist on SQLite
                    || differ.index_pairs().any(|(previous, next)| self.index_should_be_renamed(&previous, &next))
                    || differ.created_foreign_keys().next().is_some()
                    || differ.dropped_foreign_keys().next().is_some()
            })
            .map(|table| table.next().name().to_owned())
            .collect()
    }

    fn should_push_foreign_keys_from_created_tables(&self) -> bool {
        false
    }
}
