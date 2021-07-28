use super::SqlSchemaDifferFlavour;
use crate::{
    flavour::SqliteFlavour, pair::Pair, sql_schema_differ::column::ColumnTypeChange, sql_schema_differ::SqlSchemaDiffer,
};
use sql_schema_describer::{walkers::ColumnWalker, ColumnTypeFamily};
use std::collections::HashSet;

impl SqlSchemaDifferFlavour for SqliteFlavour {
    fn can_rename_index(&self) -> bool {
        false
    }

    fn column_type_change(&self, differ: Pair<ColumnWalker<'_>>) -> Option<ColumnTypeChange> {
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
                    || differ.any_column_changed()
                    || differ.created_foreign_keys().next().is_some()
                    || differ.dropped_foreign_keys().next().is_some()
            })
            .map(|table| table.next().name().to_owned())
            .collect()
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
