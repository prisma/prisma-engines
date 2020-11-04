use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::SqliteFlavour,
    pair::Pair,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use sql_schema_describer::{walkers::ColumnWalker, ColumnArity};

impl DestructiveChangeCheckerFlavour for SqliteFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &Pair<ColumnWalker<'_>>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let arity_change_is_safe = match (columns.previous().arity(), columns.next().arity()) {
            // column became required
            (ColumnArity::Nullable, ColumnArity::Required) => false,
            // column became nullable
            (ColumnArity::Required, ColumnArity::Nullable) => true,
            // nothing changed
            (ColumnArity::Required, ColumnArity::Required) | (ColumnArity::Nullable, ColumnArity::Nullable) => true,
            // not supported on SQLite
            (ColumnArity::List, _) | (_, ColumnArity::List) => unreachable!(),
        };

        if !alter_column.changes.type_changed() && arity_change_is_safe {
            return;
        }

        if alter_column.changes.arity_changed()
            && columns.next().arity().is_required()
            && columns.next().default().is_none()
        {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired {
                    table: columns.previous().table().name().to_owned(),
                    column: columns.previous().name().to_owned(),
                },
                step_index,
            );
        }

        match alter_column.type_change {
            Some(ColumnTypeChange::SafeCast) | None => (),
            Some(ColumnTypeChange::RiskyCast) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::RiskyCast {
                        table: columns.previous().table().name().to_owned(),
                        column: columns.previous().name().to_owned(),
                        previous_type: format!("{:?}", columns.previous().column_type_family()),
                        next_type: format!("{:?}", columns.next().column_type_family()),
                    },
                    step_index,
                );
            }
        }
    }

    fn check_drop_and_recreate_column(
        &self,
        _columns: &Pair<ColumnWalker<'_>>,
        _changes: &ColumnChanges,
        _plan: &mut DestructiveCheckPlan,
        _step_index: usize,
    ) {
        unreachable!("check_drop_and_recreate_column on SQLite");
    }
}
