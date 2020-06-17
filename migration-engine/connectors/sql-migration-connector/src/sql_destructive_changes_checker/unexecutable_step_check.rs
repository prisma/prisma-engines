use super::{check::Check, database_inspection_results::DatabaseInspectionResults};

#[derive(Debug)]
pub(crate) enum UnexecutableStepCheck {
    AddedRequiredFieldToTable { table: String, column: String },
    MadeOptionalFieldRequired { table: String, column: String },
    MadeScalarFieldIntoArrayField { table: String, column: String },
    // TODO:
    // AddedUnimplementableUniqueConstraint {
    //     table: String,
    //     constrained_columns: Vec<String>,
    // },
    // DeletedUsedEnumValue {
    //     r#enum: String,
    //     value: String,
    //     uses_count: Option<u64>,
    // },
    // PrimaryKeyChanged {
    //     table: String,
    // },
}

impl Check for UnexecutableStepCheck {
    fn needed_table_row_count(&self) -> Option<&str> {
        match self {
            UnexecutableStepCheck::MadeOptionalFieldRequired { table, column: _ }
            | UnexecutableStepCheck::MadeScalarFieldIntoArrayField { table, column: _ }
            | UnexecutableStepCheck::AddedRequiredFieldToTable { table, column: _ } => Some(table),
        }
    }

    fn needed_column_value_count(&self) -> Option<(&str, &str)> {
        match self {
            UnexecutableStepCheck::MadeOptionalFieldRequired { table, column }
            | UnexecutableStepCheck::MadeScalarFieldIntoArrayField { table, column } => Some((table, column)),
            UnexecutableStepCheck::AddedRequiredFieldToTable { .. } => None,
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
                let message = |details| format!("Changed the column `{column}` on the `{table}` table from a scalar field to a list field. {details}", column = column, table = table, details = details);

                match dbg!(database_checks.get_row_and_non_null_value_count(table, column)) {
                    (Some(0), _) => return None,
                    (_, Some(0)) => return None,
                    (_, Some(value_count)) => Some(message(format_args!(
                        "There are {} existing non-null values in that column, this migration step cannot be executed.", value_count
                    ))),
                    (_, _) => Some(message(format_args!(
                        "If there are non-null values in that column, this migration step will fail."
                    )))

                }
            }
            // TODO
            //
            // SqlUnexecutableMigration::AddedUnimplementableUniqueConstraint { table, constrained_columns } => write!(f, "Added a unique constraint that would not hold given existing data on `{table}`.{constrained_columns:?}", table = table, constrained_columns = constrained_columns)?,
            // SqlUnexecutableMigration::DeletedUsedEnumValue {
            //     r#enum,
            //     value,
            //     uses_count,
            // } => {
            //     write!(f, "You deleted the value `{value}` of the `{enum_name}` enum, but it is still used `{uses_count:?}` times in the database. (TODO: say which tables)", value = value, enum_name = r#enum, uses_count = uses_count)?
            // }
            // SqlUnexecutableMigration::PrimaryKeyChanged { table } => write!(
            //     f,
            //     "The id field(s) for table {table} changed. This is currently not supported by prisma
            //     migrate.",
            //     table = table
            // )?,
        }
    }
}
