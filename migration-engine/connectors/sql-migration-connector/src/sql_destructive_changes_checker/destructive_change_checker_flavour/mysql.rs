use super::DestructiveChangeCheckerFlavour;
use crate::{
    expanded_alter_column::{expand_mysql_alter_column, MysqlAlterColumn},
    flavour::MysqlFlavour,
    sql_destructive_changes_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
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
                } else {
                    plan.push_warning(SqlMigrationWarningCheck::AlterColumn {
                        table: columns.previous.table().name().to_owned(),
                        column: columns.next.name().to_owned(),
                    });
                }
            }
        }
    }
}
