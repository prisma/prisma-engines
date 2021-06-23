use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::MysqlFlavour,
    pair::Pair,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use datamodel_connector::Connector;
use sql_datamodel_connector::SqlDatamodelConnectors;
use sql_schema_describer::walkers::ColumnWalker;

impl DestructiveChangeCheckerFlavour for MysqlFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &Pair<ColumnWalker<'_>>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let AlterColumn {
            column_id: _,
            changes,
            type_change,
        } = alter_column;

        // If only the default changed, the migration is safe.
        if changes.only_default_changed() {
            return;
        }

        // Otherwise, case by case.
        // Column went from optional to required. This is unexecutable unless the table is
        // empty or the column has no existing NULLs.
        if changes.arity_changed() && columns.next().arity().is_required() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            );

            return;
        }

        if changes.only_type_changed() && is_safe_enum_change(columns, plan, step_index) {
            return;
        }

        let datamodel_connector = SqlDatamodelConnectors::mysql(false);
        let previous_type = match &columns.previous().column_type().native_type {
            Some(tpe) => datamodel_connector.render_native_type(tpe.clone()),
            _ => format!("{:?}", columns.previous().column_type_family()),
        };

        let next_type = match &columns.next().column_type().native_type {
            Some(tpe) => datamodel_connector.render_native_type(tpe.clone()),
            _ => format!("{:?}", columns.next().column_type_family()),
        };

        match type_change {
            None | Some(ColumnTypeChange::SafeCast) => (),
            Some(ColumnTypeChange::RiskyCast) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::RiskyCast {
                        table: columns.previous().table().name().to_owned(),
                        column: columns.previous().name().to_owned(),
                        previous_type,
                        next_type,
                    },
                    step_index,
                );
            }
            Some(ColumnTypeChange::NotCastable) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::NotCastable {
                        table: columns.previous().table().name().to_owned(),
                        column: columns.previous().name().to_owned(),
                        previous_type,
                        next_type,
                    },
                    step_index,
                );
            }
        };
    }

    fn check_drop_and_recreate_column(
        &self,
        columns: &Pair<ColumnWalker<'_>>,
        changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        // Unexecutable drop and recreate.
        if changes.arity_changed()
            && columns.previous().arity().is_nullable()
            && columns.next().arity().is_required()
            && columns.next().default().is_none()
        {
            plan.push_unexecutable(
                UnexecutableStepCheck::AddedRequiredFieldToTable {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            )
        } else if columns.next().arity().is_required() && columns.next().default().is_none() {
            plan.push_unexecutable(
                UnexecutableStepCheck::DropAndRecreateRequiredColumn {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            )
        } else {
            //todo this is probably due to a not castable type change. we should give that info in the warning
            plan.push_warning(
                SqlMigrationWarningCheck::DropAndRecreateColumn {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            )
        }
    }
}

/// If the type change is an enum change, diagnose it, and return whether it _was_ an enum change.
fn is_safe_enum_change(columns: &Pair<ColumnWalker<'_>>, plan: &mut DestructiveCheckPlan, step_index: usize) -> bool {
    if let (Some(previous_enum), Some(next_enum)) = (
        columns.previous().column_type_family_as_enum(),
        columns.next().column_type_family_as_enum(),
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
