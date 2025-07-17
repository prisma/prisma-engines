use super::{
    check::{Check, Column, Table},
    database_inspection_results::DatabaseInspectionResults,
};

#[derive(Debug)]
pub(crate) enum UnexecutableStepCheck {
    AddedRequiredFieldToTable(Column),
    AddedRequiredFieldToTableWithPrismaLevelDefault(Column),
    MadeOptionalFieldRequired(Column),
    MadeScalarFieldIntoArrayField(Column),
    DropAndRecreateRequiredColumn(Column),
}

impl Check for UnexecutableStepCheck {
    fn needed_table_row_count(&self) -> Option<Table> {
        match self {
            UnexecutableStepCheck::AddedRequiredFieldToTableWithPrismaLevelDefault(column)
            | UnexecutableStepCheck::MadeOptionalFieldRequired(column)
            | UnexecutableStepCheck::MadeScalarFieldIntoArrayField(column)
            | UnexecutableStepCheck::AddedRequiredFieldToTable(column)
            | UnexecutableStepCheck::DropAndRecreateRequiredColumn(column) => Some(Table::from_column(column)),
        }
    }

    fn needed_column_value_count(&self) -> Option<Column> {
        match self {
            UnexecutableStepCheck::MadeOptionalFieldRequired(column)
            | UnexecutableStepCheck::MadeScalarFieldIntoArrayField(column) => Some(column.clone()),
            UnexecutableStepCheck::AddedRequiredFieldToTable { .. }
            | UnexecutableStepCheck::AddedRequiredFieldToTableWithPrismaLevelDefault { .. }
            | UnexecutableStepCheck::DropAndRecreateRequiredColumn { .. } => None,
        }
    }

    fn evaluate<'a>(&self, database_checks: &DatabaseInspectionResults) -> Option<String> {
        match self {
            UnexecutableStepCheck::AddedRequiredFieldToTable(column) => {
                let message = |details| {
                    format!(
                        "Added the required column `{column}` to the `{table}` table without a default value. {details}",
                        table = column.table,
                        column = column.column,
                        details = details,
                    )
                };

                let message = match database_checks.get_row_count(&Table::from_column(column)) {
                    Some(0) => return None, // Adding a required column is possible if there is no data
                    Some(row_count) => message(format_args!(
                        "There are {row_count} rows in this table, it is not possible to execute this step."
                    )),
                    None => message(format_args!("This is not possible if the table is not empty.")),
                };

                Some(message)
            }
            UnexecutableStepCheck::AddedRequiredFieldToTableWithPrismaLevelDefault(column) => {
                let message = |details| {
                    format!(
                        "The required column `{column}` was added to the `{table}` table with a prisma-level default value. {details} Please add this column as optional, then populate it before making it required.",
                        table = column.table,
                        column = column.column,
                        details = details,
                    )
                };

                let message = match database_checks.get_row_count(&Table::from_column(column)) {
                    Some(0) => return None, // Adding a required column is possible if there is no data
                    Some(row_count) => message(format_args!(
                        "There are {row_count} rows in this table, it is not possible to execute this step."
                    )),
                    None => message(format_args!("This is not possible if the table is not empty.")),
                };

                Some(message)
            }
            UnexecutableStepCheck::MadeOptionalFieldRequired(column) => {
                match database_checks.get_row_and_non_null_value_count(column) {
                    (Some(0), _) => None,
                    (Some(row_count), Some(value_count)) => {
                        let null_value_count = row_count - value_count;

                        if null_value_count == 0 {
                            return None;
                        }

                        Some(format!(
                            "Made the column `{column}` on table `{table}` required, but there are {null_value_count} existing NULL values.",
                            column = column.column,
                            table = column.table,
                            null_value_count = null_value_count,
                        ))
                    }
                    (_, _) => Some(format!(
                        "Made the column `{column}` on table `{table}` required. This step will fail if there are existing NULL values in that column.",
                        column = column.column,
                        table = column.table
                    )),
                }
            }
            UnexecutableStepCheck::MadeScalarFieldIntoArrayField(column) => {
                let message = |details| {
                    format!(
                        "Changed the column `{column}` on the `{table}` table from a scalar field to a list field. {details}",
                        column = column.column,
                        table = column.table,
                        details = details
                    )
                };

                match database_checks.get_row_and_non_null_value_count(column) {
                    (Some(0), _) => None,
                    (_, Some(0)) => None,
                    (_, Some(value_count)) => Some(message(format_args!(
                        "There are {value_count} existing non-null values in that column, this step cannot be executed."
                    ))),
                    (_, _) => Some(message(format_args!(
                        "If there are non-null values in that column, this step will fail."
                    ))),
                }
            }
            UnexecutableStepCheck::DropAndRecreateRequiredColumn(column) => {
                match database_checks.get_row_count(&Table::from_column(column)) {
                    None => Some(format!(
                        "Changed the type of `{column}` on the `{table}` table. No cast exists, the column would be dropped and recreated, which cannot be done if there is data, since the column is required.",
                        column = column.column,
                        table = column.table
                    )),
                    Some(0) => None,
                    Some(_) => Some(format!(
                        "Changed the type of `{column}` on the `{table}` table. No cast exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table.",
                        column = column.column,
                        table = column.table
                    )),
                }
            }
        }
    }
}
