use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::SqliteFlavour,
    sql_destructive_changes_checker::{
        destructive_check_plan::DestructiveCheckPlan, warning_check::SqlMigrationWarning,
    },
    sql_schema_differ::ColumnDiffer,
};
use sql_schema_describer::{ColumnArity, Table};

impl DestructiveChangeCheckerFlavour for SqliteFlavour {
    fn check_alter_column(&self, previous_table: &Table, columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan) {
        let arity_change_is_safe = match (&columns.previous.tpe.arity, &columns.next.tpe.arity) {
            // column became required
            (ColumnArity::Nullable, ColumnArity::Required) => false,
            // column became nullable
            (ColumnArity::Required, ColumnArity::Nullable) => true,
            // nothing changed
            (ColumnArity::Required, ColumnArity::Required) | (ColumnArity::Nullable, ColumnArity::Nullable) => true,
            // not supported on SQLite
            (ColumnArity::List, _) | (_, ColumnArity::List) => unreachable!(),
        };

        if !columns.all_changes().type_changed() && arity_change_is_safe {
            return;
        }

        plan.push_warning(SqlMigrationWarning::AlterColumn {
            table: previous_table.name.clone(),
            column: columns.next.name.clone(),
        });
    }
}
