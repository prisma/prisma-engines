use super::database_inspection_results::DatabaseInspectionResults;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Table {
    pub table: String,
    pub namespace: Option<String>,
}

impl Table {
    pub fn new(table: String, namespace: Option<String>) -> Self {
        Self { table, namespace }
    }

    pub fn from_column(column: &Column) -> Self {
        Self {
            table: column.table.clone(),
            namespace: column.namespace.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Column {
    pub table: String,
    pub namespace: Option<String>,
    pub column: String,
}

impl Column {
    pub fn new(table: String, namespace: Option<String>, column: String) -> Self {
        Self {
            table,
            namespace,
            column,
        }
    }
}

/// This trait should be implemented by warning and unexecutable migration types. It lets them
/// describe what data they need from the current state of the database to be as accurate and
/// informative as possible.
pub(super) trait Check {
    /// Indicates that the row count for the table with the returned name should be inspected.
    fn needed_table_row_count(&self) -> Option<Table> {
        None
    }

    /// Indicates that the the number of non-null values should be inspected for the returned table and column.
    fn needed_column_value_count(&self) -> Option<Column> {
        None
    }

    /// This function will always be called for every check in a migration. Each change must check
    /// for the data it needs in the database inspection results. If there is no data, it should
    /// assume the current state of the database could not be inspected and warn with a best effort
    /// message explaining under which conditions the migration could not be applied or would cause
    /// data loss.
    ///
    /// The only case where `None` should be returned is when there is data about the current state
    /// of the database, and that data indicates that the migration step would be executable and
    /// safe.
    fn evaluate(&self, database_check_results: &DatabaseInspectionResults) -> Option<String>;
}
