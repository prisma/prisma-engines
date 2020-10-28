use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::MysqlFlavour,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use sql_schema_describer::walkers::ColumnWalker;

impl DestructiveChangeCheckerFlavour for MysqlFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        (previous, next): (&ColumnWalker<'_>, &ColumnWalker<'_>),
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let AlterColumn {
            column_name: _,
            column_index: _,
            changes,
            type_change,
        } = alter_column;

        // If only the default changed, the migration is safe.
        if changes.only_default_changed() {
            return ();
        }

        // Otherwise, case by case.
        // Column went from optional to required. This is unexecutable unless the table is
        // empty or the column has no existing NULLs.
        if changes.arity_changed() && next.arity().is_required() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired {
                    column: previous.name().to_owned(),
                    table: previous.table().name().to_owned(),
                },
                step_index,
            );

            return;
        }

        if changes.only_type_changed() && is_safe_enum_change((previous, next), plan, step_index) {
            return;
        }

        match type_change {
            Some(ColumnTypeChange::SafeCast) | None => (),
            Some(ColumnTypeChange::RiskyCast) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::RiskyCast {
                        table: previous.table().name().to_owned(),
                        column: previous.name().to_owned(),
                        previous_type: format!("{:?}", previous.column_type_family()),
                        next_type: format!("{:?}", next.column_type_family()),
                    },
                    step_index,
                );
            }
        };
    }

    fn check_drop_and_recreate_column(
        &self,
        _columns: (&ColumnWalker<'_>, &ColumnWalker<'_>),
        _changes: &ColumnChanges,
        _plan: &mut DestructiveCheckPlan,
        _step_index: usize,
    ) {
        panic!("check_drop_and_recreate_column on MySQL")
    }
}

/// If the type change is an enum change, diagnose it, and return whether it _was_ an enum change.
fn is_safe_enum_change(
    (previous, next): (&ColumnWalker<'_>, &ColumnWalker<'_>),
    plan: &mut DestructiveCheckPlan,
    step_index: usize,
) -> bool {
    if let (Some(previous_enum), Some(next_enum)) =
        (previous.column_type_family_as_enum(), next.column_type_family_as_enum())
    {
        let removed_values: Vec<String> = previous_enum
            .values
            .iter()
            .filter(|previous_value| {
                !next_enum
                    .values
                    .iter()
                    .any(|next_value| previous_value.as_str() == next_value.as_str())
            })
            .cloned()
            .collect();

        if !removed_values.is_empty() {
            plan.push_warning(
                SqlMigrationWarningCheck::EnumValueRemoval {
                    enm: next_enum.name.clone(),
                    values: removed_values,
                },
                step_index,
            );
        }

        return true;
    }

    false
}
