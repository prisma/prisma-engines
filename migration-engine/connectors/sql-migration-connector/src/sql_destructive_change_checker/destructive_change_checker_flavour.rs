mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use super::DestructiveCheckPlan;
use crate::{pair::Pair, sql_migration::AlterColumn, sql_schema_differ::ColumnChanges};
use migration_connector::{BoxFuture, ConnectorError, ConnectorResult};
use sql_schema_describer::walkers::ColumnWalker;

/// Flavour-specific destructive change checks and queries.
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

    fn count_rows_in_table<'a>(&'a mut self, table_name: &'a str) -> BoxFuture<'a, ConnectorResult<i64>>;

    fn count_values_in_column<'a>(
        &'a mut self,
        table_and_column: (&'a str, &'a str),
    ) -> BoxFuture<'a, ConnectorResult<i64>>;
}

/// Display a column type for warnings/errors.
fn display_column_type(
    column: sql_schema_describer::walkers::ColumnWalker<'_>,
    connector: &dyn psl::datamodel_connector::Connector,
) -> String {
    match &column.column_type().native_type {
        Some(tpe) => connector.introspect_native_type(tpe.clone()).to_string(),
        _ => format!("{:?}", column.column_type_family()),
    }
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
        .and_then(|value| value.as_integer())
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
        .and_then(|count| count.as_integer())
        .ok_or_else(|| ConnectorError::from_msg("Unexpected result set shape when checking dropped columns.".into()))
}
