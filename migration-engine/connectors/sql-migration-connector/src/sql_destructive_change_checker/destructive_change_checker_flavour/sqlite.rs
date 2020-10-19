use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::SqliteFlavour,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::AlterColumn,
    sql_schema_differ::{ColumnChanges, ColumnDiffer, ColumnTypeChange},
};
use sql_schema_describer::ColumnArity;

impl DestructiveChangeCheckerFlavour for SqliteFlavour {
    fn check_alter_column(
        &self,
        _alter_column: &AlterColumn,
        _columns: &ColumnDiffer<'_>,
        _plan: &mut DestructiveCheckPlan,
        _step_index: usize,
    ) {
        unreachable!("check_alter_column on SQLite");
    }

    fn check_drop_and_recreate_column(
        &self,
        columns: &ColumnDiffer<'_>,
        _changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let (changes, type_change) = columns.all_changes();

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

        if !changes.type_changed() && arity_change_is_safe {
            return;
        }

        if changes.arity_changed() && columns.next.arity().is_required() && columns.next.default().is_none() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired {
                    table: columns.previous.table().name().to_owned(),
                    column: columns.previous.name().to_owned(),
                },
                step_index,
            );
        }

        match type_change {
            Some(ColumnTypeChange::SafeCast) | None => (),
            Some(ColumnTypeChange::RiskyCast) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::RiskyCast {
                        table: columns.previous.table().name().to_owned(),
                        column: columns.previous.name().to_owned(),
                        previous_type: format!("{:?}", columns.previous.column_type_family()),
                        next_type: format!("{:?}", columns.next.column_type_family()),
                    },
                    step_index,
                );
            }
            Some(ColumnTypeChange::NotCastable) => unreachable!("uncastable change on SQLite"),
        }
    }
}
