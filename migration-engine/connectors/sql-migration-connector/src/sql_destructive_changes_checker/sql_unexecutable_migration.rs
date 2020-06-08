use super::{check::Check, database_inspection_results::DatabaseInspectionResults};

#[derive(Debug)]
pub(crate) enum SqlUnexecutableMigration {
    AddedRequiredFieldToTable { table: String, column: String },
    MadeOptionalFieldRequired { table: String, column: String },
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

impl Check for SqlUnexecutableMigration {
    fn check_row_count(&self) -> Option<&str> {
        match self {
            SqlUnexecutableMigration::MadeOptionalFieldRequired { table, column: _ }
            | SqlUnexecutableMigration::AddedRequiredFieldToTable { table, column: _ } => Some(table),
        }
    }

    fn check_existing_values(&self) -> Option<(&str, &str)> {
        match self {
            SqlUnexecutableMigration::MadeOptionalFieldRequired { table, column } => Some((table, column)),
            SqlUnexecutableMigration::AddedRequiredFieldToTable { .. } => None,
        }
    }

    fn render<'a>(&self, database_checks: &DatabaseInspectionResults) -> Option<String> {
        match self {
            SqlUnexecutableMigration::AddedRequiredFieldToTable { table, column } => {
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
                    Some(row_count) => {
                        message(format_args!(
                            "There are {row_count} rows in this table, it is not possible to execute this migration.",
                            row_count = row_count
                        ))
                    }
                    None => message(format_args!("This is not possible if the table is not empty."))

                };

                Some(message)
            }
            SqlUnexecutableMigration::MadeOptionalFieldRequired { table, column } => {
                match database_checks.get_value_count(table, column) {
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
