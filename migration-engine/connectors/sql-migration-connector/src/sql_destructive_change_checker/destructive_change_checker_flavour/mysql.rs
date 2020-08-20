use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::MysqlFlavour,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::expanded_alter_column::{expand_mysql_alter_column, MysqlAlterColumn},
    sql_schema_differ::ColumnDiffer,
};

impl DestructiveChangeCheckerFlavour for MysqlFlavour {
    fn check_alter_column(&self, columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan) {
        match expand_mysql_alter_column(columns) {
            MysqlAlterColumn::DropDefault => (), // dropping a default is safe

            // If only the default changed, the step is safe.
            MysqlAlterColumn::Modify { changes, .. } if changes.only_default_changed() => (),

            // Otherwise, case by case.
            MysqlAlterColumn::Modify { .. } => {
                // Column went from optional to required. This is unexecutable unless the table is
                // empty or the column has no existing NULLs.
                if columns.all_changes().arity_changed() && columns.next.column.tpe.arity.is_required() {
                    plan.push_unexecutable(UnexecutableStepCheck::MadeOptionalFieldRequired {
                        column: columns.previous.name().to_owned(),
                        table: columns.previous.table().name().to_owned(),
                    });

                    return;
                }

                if columns.all_changes().only_type_changed() && diagnose_enum_change(columns, plan) {
                    return;
                }

                plan.push_warning(SqlMigrationWarningCheck::AlterColumn {
                    table: columns.previous.table().name().to_owned(),
                    column: columns.next.name().to_owned(),
                });
            }
        }
    }
}

/// If the type change is an enum change, diagnose it, and return whether it _was_ an enum change.
fn diagnose_enum_change(columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan) -> bool {
    if let (Some(previous_enum), Some(next_enum)) = (
        columns.previous.column_type_family_as_enum(),
        columns.next.column_type_family_as_enum(),
    ) {
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
            plan.push_warning(SqlMigrationWarningCheck::EnumValueRemoval {
                enm: next_enum.name.clone(),
                values: removed_values,
            });
        }

        return true;
    }

    false
}
