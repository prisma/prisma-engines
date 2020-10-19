use super::{check::Check, database_inspection_results::DatabaseInspectionResults};

#[derive(Debug)]
pub(crate) enum UnexecutableStepCheck {
    AddedRequiredFieldToTable { table: String, column: String },
    MadeOptionalFieldRequired { table: String, column: String },
    MadeScalarFieldIntoArrayField { table: String, column: String },
    DropAndRecreateRequiredColumn { table: String, column: String },
}

impl Check for UnexecutableStepCheck {
    fn needed_table_row_count(&self) -> Option<&str> {
        match self {
            UnexecutableStepCheck::MadeOptionalFieldRequired { table, column: _ }
            | UnexecutableStepCheck::MadeScalarFieldIntoArrayField { table, column: _ }
            | UnexecutableStepCheck::AddedRequiredFieldToTable { table, column: _ }
            | UnexecutableStepCheck::DropAndRecreateRequiredColumn { table, column: _ } => Some(table),
        }
    }

    fn needed_column_value_count(&self) -> Option<(&str, &str)> {
        match self {
            UnexecutableStepCheck::MadeOptionalFieldRequired { table, column }
            | UnexecutableStepCheck::MadeScalarFieldIntoArrayField { table, column } => Some((table, column)),
            UnexecutableStepCheck::AddedRequiredFieldToTable { .. }
            | UnexecutableStepCheck::DropAndRecreateRequiredColumn { .. } => None,
        }
    }

    fn evaluate<'a>(&self, database_checks: &DatabaseInspectionResults) -> Option<String> {
        match self {
            UnexecutableStepCheck::AddedRequiredFieldToTable { table, column } => {
                let message = |details| {
                    format!(
                        "Added the required column `{column}` to the `{table}` table without a default value. {details}",
                        table = table,
                        column = column,
                        details = details,
                    )
                };

                let message = match database_checks.get_row_count(table) {
                    Some(0) => return None, // Adding a required column is possible if there is no data
                    Some(row_count) => message(format_args!(
                        "There are {row_count} rows in this table, it is not possible to execute this migration.",
                        row_count = row_count
                    )),
                    None => message(format_args!("This is not possible if the table is not empty.")),
                };

                Some(message)
            }
            UnexecutableStepCheck::MadeOptionalFieldRequired { table, column } => {
                match database_checks.get_row_and_non_null_value_count(table, column) {
                    (Some(0), _) => None,
                    (Some(row_count), Some(value_count)) => {
                        let null_value_count = row_count - value_count;

                        if null_value_count == 0 {
                            return None
                        }

                        Some(format!(
                            "Made the column `{column}` on table `{table}` required, but there are {null_value_count} existing NULL values.",
                            column = column,
                            table = table,
                            null_value_count = null_value_count,
                        ))
                    },
                    (_, _) => Some(format!(
                        "Made the column `{column}` on table `{table}` required. The migration will fail if there are existing NULL values in that column.",
                        column = column,
                        table = table
                    )),
                }
            }
            UnexecutableStepCheck::MadeScalarFieldIntoArrayField { table, column } => {
                let message = |details| {
                    format!("Changed the column `{column}` on the `{table}` table from a scalar field to a list field. {details}", column = column, table = table, details = details)
                };

                match database_checks.get_row_and_non_null_value_count(table, column) {
                    (Some(0), _) => None,
                    (_, Some(0)) => None,
                    (_, Some(value_count)) => Some(message(format_args!(
                        "There are {} existing non-null values in that column, this migration step cannot be executed.",
                        value_count
                    ))),
                    (_, _) => Some(message(format_args!(
                        "If there are non-null values in that column, this migration step will fail."
                    ))),
                }
            }
            UnexecutableStepCheck::DropAndRecreateRequiredColumn { table, column } => {
                match database_checks.get_row_count(table) {
                    None => Some(format!("Changed the type of `{column}` on the `{table}` table. No casts exists, the column would be dropped and recreated, which cannot be done if there is data, since the column is required.", column = column, table = table)),
                    Some(0) => None,
                    Some(_) => Some(format!("Changed the type of `{column}` on the `{table}` table. No casts exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table.", column = column, table = table)),
                }
            }
        }
    }
}
