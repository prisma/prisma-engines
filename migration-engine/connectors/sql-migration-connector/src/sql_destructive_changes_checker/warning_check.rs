use super::{check::Check, database_inspection_results::DatabaseInspectionResults};

#[derive(Debug)]
pub(super) enum SqlMigrationWarningCheck {
    NonEmptyColumnDrop { table: String, column: String },
    NonEmptyTableDrop { table: String },
    AlterColumn { table: String, column: String },
    PrimaryKeyChange { table: String },
}

impl Check for SqlMigrationWarningCheck {
    fn needed_table_row_count(&self) -> Option<&str> {
        match self {
            SqlMigrationWarningCheck::NonEmptyTableDrop { table }
            | SqlMigrationWarningCheck::PrimaryKeyChange { table } => Some(table),
            SqlMigrationWarningCheck::NonEmptyColumnDrop { .. } | SqlMigrationWarningCheck::AlterColumn { .. } => None,
        }
    }

    fn needed_column_value_count(&self) -> Option<(&str, &str)> {
        match self {
            SqlMigrationWarningCheck::NonEmptyColumnDrop { table, column }
            | SqlMigrationWarningCheck::AlterColumn { table, column } => Some((table, column)),

            SqlMigrationWarningCheck::NonEmptyTableDrop { .. } | SqlMigrationWarningCheck::PrimaryKeyChange { .. } => {
                None
            }
        }
    }

    fn evaluate(&self, database_check_results: &DatabaseInspectionResults) -> Option<String> {
        match self {
            SqlMigrationWarningCheck::NonEmptyTableDrop { table } => match database_check_results.get_row_count(table) {
                Some(0) => None, // dropping the table is safe if it's empty
                Some(rows_count) => Some(format!("You are about to drop the `{table_name}` table, which is not empty ({rows_count} rows).", table_name = table, rows_count = rows_count)),
                None => Some(format!("You are about to drop the `{}` table. If the table is not empty, all the data it contains will be lost.", table)),
            },
            SqlMigrationWarningCheck::NonEmptyColumnDrop { table, column } => match database_check_results.get_row_and_non_null_value_count(table, column) {
                (Some(0), _) => None, // it's safe to drop a column on an empty table
                (_, Some(0)) => None, // it's safe to drop a column if it only contains null values
                (_, Some(value_count)) => Some(format!("You are about to drop the column `{column_name}` on the `{table_name}` table, which still contains {value_count} non-null values.", column_name = column, table_name = table, value_count = value_count)),
                (_, _) => Some(format!("You are about to drop the column `{column_name}` on the `{table_name}` table. All the data in the column will be lost.", column_name = column, table_name = table)),
            },
            SqlMigrationWarningCheck::AlterColumn { table, column } => match database_check_results.get_row_and_non_null_value_count(table, column) {
                (Some(0), _) => None, // it's safe to alter a column on an empty table
                (_, Some(0)) => None, // it's safe to alter a column if it only contains null values
                (_, Some(value_count)) => Some(format!("You are about to alter the column `{column_name}` on the `{table_name}` table, which still contains {value_count} non-null values. The data in that column could be lost.", column_name = column, table_name = table, value_count = value_count)),
                (_, _) => Some(format!("You are about to alter the column `{column_name}` on the `{table_name}` table. The data in that column could be lost.", column_name = column, table_name = table)),

            },
            SqlMigrationWarningCheck::PrimaryKeyChange { table } => match database_check_results.get_row_count(table) {
                Some(0) => None,
                _ => Some(format!("The migration will change the primary key for the `{table}` table. If it partially fails, the table could be left without primary key constraint.", table = table)),
            }
        }
    }
}
