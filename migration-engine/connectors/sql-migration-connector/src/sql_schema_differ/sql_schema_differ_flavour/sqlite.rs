use sql_schema_describer::ColumnTypeFamily;

use super::SqlSchemaDifferFlavour;
use crate::{
    flavour::SqliteFlavour,
    sql_schema_differ::SqlSchemaDiffer,
    sql_schema_differ::{
        column::{ColumnDiffer, ColumnTypeChange},
        differ_database::DifferDatabase,
    },
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

    fn tables_to_redefine(&self, db: &DifferDatabase<'_>) -> HashSet<String> {
        db.table_pairs()
            .filter(|table| {
                db.created_primary_key(table).is_some()
                    || db.dropped_primary_key(table).is_some()
                    || db.dropped_columns(table).next().is_some()
                    || db.added_columns(table).any(|col| col.arity().is_required())
                    || db.column_pairs(table).any(|columns| columns.all_changes().0.differs_in_something())
                    // ALTER INDEX does not exist on SQLite
                    || db.index_pairs(table).any(|pair| self.index_should_be_renamed(&pair))
                    || db.created_foreign_keys(table).next().is_some()
                    || db.dropped_foreign_keys(table).next().is_some()
            })
            .map(|table| table.next().name().to_owned())
            .collect()
    }

    fn should_push_foreign_keys_from_created_tables(&self) -> bool {
        false
    }
}
