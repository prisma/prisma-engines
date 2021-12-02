use super::{check::Check, database_inspection_results::DatabaseInspectionResults};

#[derive(Debug)]
pub(super) enum SqlMigrationWarningCheck {
    DropAndRecreateColumn {
        table: String,
        column: String,
    },
    NonEmptyColumnDrop {
        table: String,
        column: String,
    },
    NonEmptyTableDrop {
        table: String,
    },
    RiskyCast {
        table: String,
        column: String,
        previous_type: String,
        next_type: String,
    },
    NotCastable {
        table: String,
        column: String,
        previous_type: String,
        next_type: String,
    },
    PrimaryKeyChange {
        table: String,
    },
    UniqueConstraintAddition {
        table: String,
        columns: Vec<String>,
    },
    EnumValueRemoval {
        enm: String,
        values: Vec<String>,
    },
}

impl Check for SqlMigrationWarningCheck {
    fn needed_table_row_count(&self) -> Option<&str> {
        match self {
            SqlMigrationWarningCheck::NonEmptyTableDrop { table }
            | SqlMigrationWarningCheck::PrimaryKeyChange { table }
            | SqlMigrationWarningCheck::DropAndRecreateColumn { table, column: _ } => Some(table),
            SqlMigrationWarningCheck::NonEmptyColumnDrop { .. } | SqlMigrationWarningCheck::RiskyCast { .. } => None,
            _ => None,
        }
    }

    fn needed_column_value_count(&self) -> Option<(&str, &str)> {
        match self {
            SqlMigrationWarningCheck::NonEmptyColumnDrop { table, column }
            | SqlMigrationWarningCheck::RiskyCast { table, column, .. }
            | SqlMigrationWarningCheck::DropAndRecreateColumn { table, column } => Some((table, column)),

            SqlMigrationWarningCheck::NonEmptyTableDrop { .. } | SqlMigrationWarningCheck::PrimaryKeyChange { .. } => {
                None
            }
            _ => None,
        }
    }

    fn evaluate(&self, database_check_results: &DatabaseInspectionResults) -> Option<String> {
        match self {
            SqlMigrationWarningCheck::DropAndRecreateColumn { table, column } => {
                match database_check_results.get_row_and_non_null_value_count(table, column) {
                (Some(0), _) => None,
                (_, Some(0)) => None,
                (_, None) => Some(format!("The `{}` column on the `{}` table would be dropped and recreated. This will lead to data loss if there is data in the column.", column, table)),
                (_, Some(_row_count)) => Some(format!("The `{}` column on the `{}` table would be dropped and recreated. This will lead to data loss.", column, table)),

            }
        },
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
            SqlMigrationWarningCheck::RiskyCast { table, column, previous_type, next_type } => match database_check_results.get_row_and_non_null_value_count(table, column) {
                (Some(0), _) => None, // it's safe to alter a column on an empty table
                (_, Some(0)) => None, // it's safe to alter a column if it only contains null values
                (_, Some(value_count)) => Some(format!("You are about to alter the column `{column_name}` on the `{table_name}` table, which contains {value_count} non-null values. The data in that column will be cast from `{old_type}` to `{new_type}`.", column_name = column, table_name = table, value_count = value_count, old_type = previous_type, new_type = next_type)),
                (_, _) => Some(format!("You are about to alter the column `{column_name}` on the `{table_name}` table. The data in that column could be lost. The data in that column will be cast from `{old_type}` to `{new_type}`.", column_name = column, table_name = table, old_type = previous_type, new_type = next_type)),

            },

            // todo this seems to not be reached when only a table is dropped and recreated
            SqlMigrationWarningCheck::NotCastable { table, column, previous_type, next_type } => match database_check_results.get_row_and_non_null_value_count(table, column) {
                (Some(0), _) => None, // it's safe to alter a column on an empty table
                (_, Some(0)) => None, // it's safe to alter a column if it only contains null values
                (_, Some(value_count)) => Some(format!("You are about to alter the column `{column_name}` on the `{table_name}` table, which contains {value_count} non-null values. The data in that column will be cast from `{old_type}` to `{new_type}`. This cast may fail. Please make sure the data in the column can be cast.", column_name = column, table_name = table, value_count = value_count, old_type = previous_type, new_type = next_type)),
                (_, _) => Some(format!("You are about to alter the column `{column_name}` on the `{table_name}` table. The data in that column will be cast from `{old_type}` to `{new_type}`. This cast may fail. Please make sure the data in the column can be cast.", column_name = column, table_name = table, old_type = previous_type, new_type = next_type)),

            },
            SqlMigrationWarningCheck::PrimaryKeyChange { table } => match database_check_results.get_row_count(table) {
                Some(0) => None,
                _ => Some(format!("The primary key for the `{table}` table will be changed. If it partially fails, the table could be left without primary key constraint.", table = table)),
            },
            SqlMigrationWarningCheck::UniqueConstraintAddition { table, columns } =>  Some(format!("A unique constraint covering the columns `[{columns}]` on the table `{table}` will be added. If there are existing duplicate values, this will fail.", table = table, columns = columns.join(","))),
            SqlMigrationWarningCheck::EnumValueRemoval { enm, values } =>  Some(format!("The values [{values}] on the enum `{enm}` will be removed. If these variants are still used in the database, this will fail.", enm = enm, values = values.join(","))),

        }
    }
}
