use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::SqlConnectorFlavour,
    migration_pair::MigrationPair,
    sql_destructive_change_checker::{
        check::{Column, Table},
        destructive_check_plan::DestructiveCheckPlan,
        unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use schema_connector::{BoxFuture, ConnectorResult};
use sql_schema_describer::walkers::TableColumnWalker;

#[derive(Debug, Default)]
pub struct MysqlDestructiveChangeCheckerFlavour;

impl MysqlDestructiveChangeCheckerFlavour {
    fn datamodel_connector(&self) -> &dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::MYSQL
    }
}

impl DestructiveChangeCheckerFlavour for MysqlDestructiveChangeCheckerFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &MigrationPair<TableColumnWalker<'_>>,
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
        if changes.arity_changed() && columns.next.arity().is_required() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            );

            return;
        }

        if changes.only_type_changed() && is_safe_enum_change(columns, plan, step_index) {
            return;
        }

        let previous_type = super::display_column_type(columns.previous, self.datamodel_connector());
        let next_type = super::display_column_type(columns.next, self.datamodel_connector());

        match type_change {
            None | Some(ColumnTypeChange::SafeCast) => (),
            Some(ColumnTypeChange::RiskyCast) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::RiskyCast {
                        table: columns.previous.table().name().to_owned(),
                        namespace: None,
                        column: columns.previous.name().to_owned(),
                        previous_type,
                        next_type,
                    },
                    step_index,
                );
            }
            Some(ColumnTypeChange::NotCastable) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::NotCastable {
                        table: columns.previous.table().name().to_owned(),
                        namespace: None,
                        column: columns.previous.name().to_owned(),
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
        columns: &MigrationPair<TableColumnWalker<'_>>,
        changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        // Unexecutable drop and recreate.
        if changes.arity_changed()
            && columns.previous.arity().is_nullable()
            && columns.next.arity().is_required()
            && columns.next.default().is_none()
        {
            plan.push_unexecutable(
                UnexecutableStepCheck::AddedRequiredFieldToTable(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            )
        } else if columns.next.arity().is_required() && columns.next.default().is_none() {
            plan.push_unexecutable(
                UnexecutableStepCheck::DropAndRecreateRequiredColumn(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            )
        } else {
            //todo this is probably due to a not castable type change. we should give that info in the warning
            plan.push_warning(
                SqlMigrationWarningCheck::DropAndRecreateColumn {
                    column: columns.previous.name().to_owned(),
                    table: columns.previous.table().name().to_owned(),
                    namespace: None,
                },
                step_index,
            )
        }
    }

    fn count_rows_in_table<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnectorFlavour,
        table: &'a Table,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        // TODO(MultiSchema): replace this when implementing MySQL.
        let query = format!("SELECT COUNT(*) FROM `{}`", table.table);

        Box::pin(async move {
            query_with_backoff(connector, &query)
                .await
                .and_then(|result_set| super::extract_table_rows_count(table, result_set))
        })
    }

    fn count_values_in_column<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnectorFlavour,
        column: &'a Column,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        // TODO(MultiSchema): replace this when implementing MySQL.
        let query = format!(
            "SELECT COUNT(*) FROM `{}` WHERE `{}` IS NOT NULL",
            column.table, column.column
        );

        Box::pin(async move {
            query_with_backoff(connector, &query)
                .await
                .and_then(super::extract_column_values_count)
        })
    }
}

/// Run the query with exponential backoff on error, from 400ms up to (400 × 2⁵)ms.
///
/// This is necessary because destructive change checks can come after a migration, and _on
/// Vitess_, schema changes are asynchronous, they can take time to take effect. That causes
/// failures in destructive change checks. Trying again later, in this case, works.
async fn query_with_backoff(
    flavour: &mut dyn SqlConnectorFlavour,
    query: &str,
) -> ConnectorResult<quaint::prelude::ResultSet> {
    let delay = std::time::Duration::from_millis(400);
    let mut result = flavour.query_raw(query, &[]).await;

    for i in 0..6 {
        match &result {
            Ok(_result_set) => break,
            Err(_) => tokio::time::sleep(delay.saturating_mul(2 ^ i)).await,
        }

        result = flavour.query_raw(query, &[]).await
    }

    result
}

/// If the type change is an enum change, diagnose it, and return whether it _was_ an enum change.
fn is_safe_enum_change(
    columns: &MigrationPair<TableColumnWalker<'_>>,
    plan: &mut DestructiveCheckPlan,
    step_index: usize,
) -> bool {
    if let (Some(previous_enum), Some(next_enum)) = (
        columns.previous.column_type_family_as_enum(),
        columns.next.column_type_family_as_enum(),
    ) {
        let removed_values: Vec<String> = previous_enum
            .values()
            .filter(|previous_value| !next_enum.values().any(|next_value| *previous_value == next_value))
            .map(ToOwned::to_owned)
            .collect();

        if !removed_values.is_empty() {
            plan.push_warning(
                SqlMigrationWarningCheck::EnumValueRemoval {
                    enm: next_enum.name().to_owned(),
                    values: removed_values,
                },
                step_index,
            );
        }

        return true;
    }

    false
}
