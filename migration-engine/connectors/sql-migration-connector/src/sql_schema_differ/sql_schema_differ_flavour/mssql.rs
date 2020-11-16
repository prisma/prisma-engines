use super::SqlSchemaDifferFlavour;
use crate::sql_schema_differ::column::ColumnDiffer;
use crate::sql_schema_differ::column::ColumnTypeChange;
use crate::{flavour::MssqlFlavour, sql_schema_differ::SqlSchemaDiffer};
use sql_schema_describer::walkers::IndexWalker;
use sql_schema_describer::ColumnTypeFamily;
use std::collections::HashSet;

impl SqlSchemaDifferFlavour for MssqlFlavour {
    fn should_skip_index_for_new_table(&self, index: &IndexWalker<'_>) -> bool {
        index.index_type().is_unique()
    }

    fn should_recreate_indexes_from_recreated_columns(&self) -> bool {
        true
    }

    fn tables_to_redefine(&self, differ: &SqlSchemaDiffer<'_>) -> HashSet<String> {
        let autoincrement_changed = differ
            .table_pairs()
            .filter(|differ| differ.column_pairs().any(|c| c.autoincrement_changed()))
            .map(|table| table.next().name().to_owned());

        let all_columns_of_the_table_gets_dropped = differ
            .table_pairs()
            .filter(|tables| {
                tables.column_pairs().all(|columns| {
                    let type_changed = columns.previous.column_type_family() != columns.next.column_type_family();
                    let not_castable = matches!(type_change_riskyness(&columns), ColumnTypeChange::NotCastable);

                    type_changed && not_castable
                })
            })
            .map(|tables| tables.previous().name().to_string());

        autoincrement_changed
            .chain(all_columns_of_the_table_gets_dropped)
            .collect()
    }

    fn column_type_change(&self, differ: &ColumnDiffer<'_>) -> Option<ColumnTypeChange> {
        if differ.previous.column_type_family() == differ.next.column_type_family() {
            None
        } else {
            Some(type_change_riskyness(differ))
        }
    }
}

fn type_change_riskyness(differ: &ColumnDiffer<'_>) -> ColumnTypeChange {
    match (differ.previous.column_type_family(), differ.next.column_type_family()) {
        (_, ColumnTypeFamily::String) => ColumnTypeChange::SafeCast,
        (ColumnTypeFamily::String, ColumnTypeFamily::Int)
        | (ColumnTypeFamily::DateTime, ColumnTypeFamily::Float)
        | (ColumnTypeFamily::String, ColumnTypeFamily::Float) => ColumnTypeChange::NotCastable,
        (_, _) => ColumnTypeChange::RiskyCast,
    }
}
