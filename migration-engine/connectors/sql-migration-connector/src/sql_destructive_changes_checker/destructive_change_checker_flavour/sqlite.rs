use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::SqliteFlavour,
    sql_destructive_changes_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarning,
    },
    sql_schema_differ::ColumnDiffer,
};
use sql_schema_describer::{ColumnArity, Table};

impl DestructiveChangeCheckerFlavour for SqliteFlavour {
    fn check_alter_column(&self, previous_table: &Table, columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan) {
        let arity_change_is_safe = match (&columns.previous.arity(), &columns.next.arity()) {
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

        if columns.all_changes().arity_changed()
            && columns.next.arity().is_required()
            && columns.next.default().is_none()
        {
            plan.push_unexecutable(UnexecutableStepCheck::MadeOptionalFieldRequired {
                table: previous_table.name.clone(),
                column: columns.previous.name().to_owned(),
            });
        }

        plan.push_warning(SqlMigrationWarning::AlterColumn {
            table: previous_table.name.clone(),
            column: columns.next.name().to_owned(),
        });
    }
}
