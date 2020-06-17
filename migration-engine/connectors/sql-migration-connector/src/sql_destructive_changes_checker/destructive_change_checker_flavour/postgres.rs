use super::DestructiveChangeCheckerFlavour;
use crate::{
    expanded_alter_column::{expand_postgres_alter_column, PostgresAlterColumn},
    flavour::PostgresFlavour,
    sql_destructive_changes_checker::{
        destructive_check_plan::DestructiveCheckPlan, warning_check::SqlMigrationWarning,
    },
    sql_schema_differ::ColumnDiffer,
};
use sql_schema_describer::Table;

impl DestructiveChangeCheckerFlavour for PostgresFlavour {
    fn check_alter_column(&self, previous_table: &Table, columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan) {
        let expanded = expand_postgres_alter_column(columns);

        // We keep the match here to keep the exhaustiveness checking for when we add variants.
        if let Some(steps) = expanded {
            let mut is_safe = true;

            for step in steps {
                match step {
                    PostgresAlterColumn::SetDefault(_)
                    | PostgresAlterColumn::DropDefault
                    | PostgresAlterColumn::DropNotNull => (),
                    PostgresAlterColumn::SetType(_) => is_safe = false,
                }
            }

            if is_safe {
                return;
            }
        }

        plan.push_warning(SqlMigrationWarning::AlterColumn {
            table: previous_table.name.clone(),
            column: columns.next.name.clone(),
        });
    }
}
