use super::{
    check::{Check, Column, Table},
    database_inspection_results::DatabaseInspectionResults,
};

#[derive(Debug)]
pub(crate) enum SqlMigrationWarningCheck {
    DropAndRecreateColumn {
        table: String,
        namespace: Option<String>,
        column: String,
    },
    NonEmptyColumnDrop {
        table: String,
        namespace: Option<String>,
        column: String,
    },
    NonEmptyTableDrop {
        table: String,
        namespace: Option<String>,
    },
    RiskyCast {
        table: String,
        namespace: Option<String>,
        column: String,
        previous_type: String,
        next_type: String,
    },
    NotCastable {
        table: String,
        namespace: Option<String>,
        column: String,
        previous_type: String,
        next_type: String,
    },
    PrimaryKeyChange {
        table: String,
        namespace: Option<String>,
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
    fn needed_table_row_count(&self) -> Option<Table> {
        match self {
            SqlMigrationWarningCheck::NonEmptyTableDrop { table, namespace }
            | SqlMigrationWarningCheck::PrimaryKeyChange { table, namespace }
            | SqlMigrationWarningCheck::DropAndRecreateColumn {
                table,
                column: _,
                namespace,
            } => Some(Table {
                table: table.clone(),
                namespace: namespace.clone(),
            }),
            SqlMigrationWarningCheck::NonEmptyColumnDrop { .. } | SqlMigrationWarningCheck::RiskyCast { .. } => None,
            _ => None,
        }
    }

    fn needed_column_value_count(&self) -> Option<Column> {
        match self {
            SqlMigrationWarningCheck::NonEmptyColumnDrop {
                table,
                column,
                namespace,
            }
            | SqlMigrationWarningCheck::RiskyCast {
                table,
                column,
                namespace,
                ..
            }
            | SqlMigrationWarningCheck::DropAndRecreateColumn {
                table,
                column,
                namespace,
            } => Some(Column {
                table: table.clone(),
                namespace: namespace.clone(),
                column: column.clone(),
            }),

            SqlMigrationWarningCheck::NonEmptyTableDrop { .. } | SqlMigrationWarningCheck::PrimaryKeyChange { .. } => {
                None
            }
            _ => None,
        }
    }

    fn evaluate(&self, database_check_results: &DatabaseInspectionResults) -> Option<String> {
        match self {
            SqlMigrationWarningCheck::DropAndRecreateColumn {
                table,
                column,
                namespace,
            } => {
                match database_check_results.get_row_and_non_null_value_count(&Column {
                    table: table.clone(),
                    namespace: namespace.clone(),
                    column: column.clone(),
                }) {
                    (Some(0), _) => None,
                    (_, Some(0)) => None,
                    (_, None) => Some(format!(
                        "The `{column}` column on the `{table}` table would be dropped and recreated. This will lead to data loss if there is data in the column."
                    )),
                    (_, Some(_row_count)) => Some(format!(
                        "The `{column}` column on the `{table}` table would be dropped and recreated. This will lead to data loss."
                    )),
                }
            }
            SqlMigrationWarningCheck::NonEmptyTableDrop { table, namespace } => match database_check_results
                .get_row_count(&Table {
                    table: table.clone(),
                    namespace: namespace.clone(),
                }) {
                Some(0) => None, // dropping the table is safe if it's empty
                Some(rows_count) => Some(format!(
                    "You are about to drop the `{table}` table, which is not empty ({rows_count} rows)."
                )),
                None => Some(format!(
                    "You are about to drop the `{table}` table. If the table is not empty, all the data it contains will be lost."
                )),
            },
            SqlMigrationWarningCheck::NonEmptyColumnDrop {
                table,
                column,
                namespace,
            } => match database_check_results.get_row_and_non_null_value_count(&Column {
                table: table.clone(),
                namespace: namespace.clone(),
                column: column.clone(),
            }) {
                (Some(0), _) => None, // it's safe to drop a column on an empty table
                (_, Some(0)) => None, // it's safe to drop a column if it only contains null values
                (_, Some(value_count)) => Some(format!(
                    "You are about to drop the column `{column}` on the `{table}` table, which still contains {value_count} non-null values."
                )),
                (_, _) => Some(format!(
                    "You are about to drop the column `{column}` on the `{table}` table. All the data in the column will be lost."
                )),
            },
            SqlMigrationWarningCheck::RiskyCast {
                table,
                column,
                previous_type,
                next_type,
                namespace,
            } => match database_check_results.get_row_and_non_null_value_count(&Column {
                table: table.clone(),
                namespace: namespace.clone(),
                column: column.clone(),
            }) {
                (Some(0), _) => None, // it's safe to alter a column on an empty table
                (_, Some(0)) => None, // it's safe to alter a column if it only contains null values
                (_, Some(value_count)) => Some(format!(
                    "You are about to alter the column `{column}` on the `{table}` table, which contains {value_count} non-null values. The data in that column will be cast from `{previous_type}` to `{next_type}`."
                )),
                (_, _) => Some(format!(
                    "You are about to alter the column `{column}` on the `{table}` table. The data in that column could be lost. The data in that column will be cast from `{previous_type}` to `{next_type}`."
                )),
            },

            // todo this seems to not be reached when only a table is dropped and recreated
            SqlMigrationWarningCheck::NotCastable {
                table,
                column,
                previous_type,
                next_type,
                namespace,
            } => match database_check_results.get_row_and_non_null_value_count(&Column {
                table: table.clone(),
                namespace: namespace.clone(),
                column: column.clone(),
            }) {
                (Some(0), _) => None, // it's safe to alter a column on an empty table
                (_, Some(0)) => None, // it's safe to alter a column if it only contains null values
                (_, Some(value_count)) => Some(format!(
                    "You are about to alter the column `{column}` on the `{table}` table, which contains {value_count} non-null values. The data in that column will be cast from `{previous_type}` to `{next_type}`. This cast may fail. Please make sure the data in the column can be cast."
                )),
                (_, _) => Some(format!(
                    "You are about to alter the column `{column}` on the `{table}` table. The data in that column will be cast from `{previous_type}` to `{next_type}`. This cast may fail. Please make sure the data in the column can be cast."
                )),
            },
            SqlMigrationWarningCheck::PrimaryKeyChange { table, namespace } => match database_check_results
                .get_row_count(&Table {
                    table: table.clone(),
                    namespace: namespace.clone(),
                }) {
                Some(0) => None,
                _ => Some(format!(
                    "The primary key for the `{table}` table will be changed. If it partially fails, the table could be left without primary key constraint."
                )),
            },
            SqlMigrationWarningCheck::UniqueConstraintAddition { table, columns } => Some(format!(
                "A unique constraint covering the columns `[{columns}]` on the table `{table}` will be added. If there are existing duplicate values, this will fail.",
                table = table,
                columns = columns.join(",")
            )),
            SqlMigrationWarningCheck::EnumValueRemoval { enm, values } => Some(format!(
                "The values [{values}] on the enum `{enm}` will be removed. If these variants are still used in the database, this will fail.",
                enm = enm,
                values = values.join(",")
            )),
        }
    }
}
