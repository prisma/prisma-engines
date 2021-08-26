mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use super::DestructiveCheckPlan;
use crate::{connection_wrapper::Connection, pair::Pair, sql_migration::AlterColumn, sql_schema_differ::ColumnChanges};
use migration_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::walkers::ColumnWalker;

/// Flavour-specific destructive change checks and queries.
#[async_trait::async_trait]
pub(crate) trait DestructiveChangeCheckerFlavour {
    /// Check for potential destructive or unexecutable alter column steps.
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &Pair<ColumnWalker<'_>>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    );

    /// Check a DropAndRecreateColumn step.
    fn check_drop_and_recreate_column(
        &self,
        columns: &Pair<ColumnWalker<'_>>,
        changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    );

    async fn count_rows_in_table(&self, table_name: &str, conn: &Connection) -> ConnectorResult<i64>;

    async fn count_values_in_column(&self, table_and_column: (&str, &str), conn: &Connection) -> ConnectorResult<i64>;
}

fn extract_table_rows_count(table_name: &str, result_set: quaint::prelude::ResultSet) -> ConnectorResult<i64> {
    result_set
        .first()
        .ok_or_else(|| {
            ConnectorError::from_msg(format!(
                "No row was returned when checking for existing rows in the `{}` table.",
                table_name
            ))
        })?
        .at(0)
        .and_then(|value| value.as_i64())
        .ok_or_else(|| {
            ConnectorError::from_msg(format!(
                "No count was returned when checking for existing rows in the `{}` table.",
                table_name
            ))
        })
}

fn extract_column_values_count(result_set: quaint::prelude::ResultSet) -> ConnectorResult<i64> {
    result_set
        .first()
        .as_ref()
        .and_then(|row| row.at(0))
        .and_then(|count| count.as_i64())
        .ok_or_else(|| ConnectorError::from_msg("Unexpected result set shape when checking dropped columns.".into()))
}
