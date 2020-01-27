#[derive(Debug)]
pub(crate) enum SqlUnexecutableMigration {
    AddedRequiredFieldToTable {
        table: String,
        column: String,
        rows_count: Option<u64>,
    },
    MadeOptionalFieldRequired {
        table: String,
        column: String,
    },
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

impl std::fmt::Display for SqlUnexecutableMigration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlUnexecutableMigration::AddedRequiredFieldToTable { table, column, rows_count } => {
                write!(f, "Added the required column `{column}` to the `{table}` table without a default value. There are {rows_count:?} rows in this table, it is not possible.", table = table, column = column, rows_count = rows_count)?
            },
            SqlUnexecutableMigration::MadeOptionalFieldRequired { table, column } => {
                write!(f, "Made the column `{column}` on table `{table}` required, but there are existing NULL values.", column = column, table = table)?
            },
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
            //     "The id field(s) for table {table} changed. This is currently not supported by prisma.",
            //     table = table
            // )?,
        }

        Ok(())
    }
}
