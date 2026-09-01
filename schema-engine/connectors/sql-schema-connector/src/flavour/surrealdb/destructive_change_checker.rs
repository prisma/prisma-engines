use crate::{
    flavour::SqlConnector,
    migration_pair::MigrationPair,
    sql_destructive_change_checker::{
        DestructiveChangeCheckerFlavour,
        check::{Column, Table},
        destructive_check_plan::DestructiveCheckPlan,
        unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use schema_connector::{BoxFuture, ConnectorError, ConnectorResult};
use sql_schema_describer::{ColumnArity, walkers::TableColumnWalker};

#[derive(Debug, Default)]
pub struct SurrealDbDestructiveChangeCheckerFlavour;

impl DestructiveChangeCheckerFlavour for SurrealDbDestructiveChangeCheckerFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &MigrationPair<TableColumnWalker<'_>>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let arity_change_is_safe = match (columns.previous.arity(), columns.next.arity()) {
            (ColumnArity::Nullable, ColumnArity::Required) => false,
            (ColumnArity::Required, ColumnArity::Nullable) => true,
            (ColumnArity::Required, ColumnArity::Required) | (ColumnArity::Nullable, ColumnArity::Nullable) => true,
            (ColumnArity::List, _) | (_, ColumnArity::List) => unreachable!(),
        };

        if !alter_column.changes.type_changed() && arity_change_is_safe {
            return;
        }

        if alter_column.changes.arity_changed()
            && columns.next.arity().is_required()
            && columns.next.default().is_none()
        {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: None,
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            );
        }

        match alter_column.type_change {
            Some(ColumnTypeChange::SafeCast) | None => (),
            Some(ColumnTypeChange::RiskyCast) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::RiskyCast {
                        table: columns.previous.table().name().to_owned(),
                        namespace: None,
                        column: columns.previous.name().to_owned(),
                        previous_type: format!("{:?}", columns.previous.column_type_family()),
                        next_type: format!("{:?}", columns.next.column_type_family()),
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
                        previous_type: format!("{:?}", columns.previous.column_type_family()),
                        next_type: format!("{:?}", columns.next.column_type_family()),
                    },
                    step_index,
                );
            }
        }
    }

    fn check_drop_and_recreate_column(
        &self,
        columns: &MigrationPair<TableColumnWalker<'_>>,
        changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        if changes.arity_changed() && columns.next.arity().is_required() && columns.next.default().is_none() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: None,
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            );
        }
    }

    fn count_rows_in_table<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnector,
        table: &'a Table,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        Box::pin(async move {
            let sql = format!("SELECT count() FROM `{}` GROUP ALL", table.table);
            let result = connector.query_raw(&sql, &[]).await?;
            let count = result
                .into_iter()
                .next()
                .and_then(|row| row.get("count").and_then(|v| v.as_integer()))
                .unwrap_or(0);
            Ok(count)
        })
    }

    fn count_values_in_column<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnector,
        column: &'a Column,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        Box::pin(async move {
            let sql = format!(
                "SELECT count() FROM `{}` WHERE `{}` IS NOT NONE GROUP ALL",
                column.table, column.column
            );
            let result = connector.query_raw(&sql, &[]).await?;
            let count = result
                .into_iter()
                .next()
                .and_then(|row| row.get("count").and_then(|v| v.as_integer()))
                .unwrap_or(0);
            Ok(count)
        })
    }
}
