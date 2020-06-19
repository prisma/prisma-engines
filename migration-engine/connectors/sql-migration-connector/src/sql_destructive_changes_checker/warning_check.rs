use super::{check::Check, database_inspection_results::DatabaseInspectionResults};

#[derive(Debug)]
pub(super) enum SqlMigrationWarning {
    NonEmptyColumnDrop { table: String, column: String },
    NonEmptyTableDrop { table: String },
    AlterColumn { table: String, column: String },
    ForeignKeyDefaultValueRemoved { table: String, column: String },
}

impl Check for SqlMigrationWarning {
    fn needed_table_row_count(&self) -> Option<&str> {
        match self {
            SqlMigrationWarning::NonEmptyTableDrop { table } => Some(table),
            SqlMigrationWarning::NonEmptyColumnDrop { .. }
            | SqlMigrationWarning::AlterColumn { .. }
            | SqlMigrationWarning::ForeignKeyDefaultValueRemoved { .. } => None,
        }
    }

    fn needed_column_value_count(&self) -> Option<(&str, &str)> {
        match self {
            SqlMigrationWarning::NonEmptyColumnDrop { table, column }
            | SqlMigrationWarning::AlterColumn { table, column } => Some((table, column)),
            SqlMigrationWarning::ForeignKeyDefaultValueRemoved { .. }
            | SqlMigrationWarning::NonEmptyTableDrop { .. } => None,
        }
    }

    fn evaluate(&self, database_check_results: &DatabaseInspectionResults) -> Option<String> {
        match self {
            SqlMigrationWarning::NonEmptyTableDrop { table } => match database_check_results.get_row_count(table) {
                Some(0) => None, // dropping the table is safe if it's empty
                Some(rows_count) => Some(format!("You are about to drop the `{table_name}` table, which is not empty ({rows_count} rows).", table_name = table, rows_count = rows_count)),
                None => Some(format!("You are about to drop the `{}` table. If the table is not empty, all the data it contains will be lost.", table)),
            },
            SqlMigrationWarning::NonEmptyColumnDrop { table, column } => match database_check_results.get_row_and_non_null_value_count(table, column) {
                (Some(0), _) => None, // it's safe to drop a column on an empty table
                (_, Some(0)) => None, // it's safe to drop a column if it only contains null values
                (_, Some(value_count)) => Some(format!("You are about to drop the column `{column_name}` on the `{table_name}` table, which still contains {value_count} non-null values.", column_name = column, table_name = table, value_count = value_count)),
                (_, _) => Some(format!("You are about to drop the column `{column_name}` on the `{table_name}` table. All the data in the column will be lost.", column_name = column, table_name = table)),
            },
            SqlMigrationWarning::AlterColumn { table, column } => match database_check_results.get_row_and_non_null_value_count(table, column) {
                (Some(0), _) => None, // it's safe to alter a column on an empty table
                (_, Some(0)) => None, // it's safe to alter a column if it only contains null values
                (_, Some(value_count)) => Some(format!("You are about to alter the column `{column_name}` on the `{table_name}` table, which still contains {value_count} non-null values. The data in that column will be lost.", column_name = column, table_name = table, value_count = value_count)),
                (_, _) => Some(format!("You are about to alter the column `{column_name}` on the `{table_name}` table. The data in that column could be lost.", column_name = column, table_name = table)),

            },
            SqlMigrationWarning::ForeignKeyDefaultValueRemoved { table, column } => Some(format!("The migration is about to remove a default value on the foreign key field `{}.{}`.", table, column)),
        }
    }
}
