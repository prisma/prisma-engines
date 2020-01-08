use pretty_assertions::assert_eq;
use sql_schema_describer::{Column, ForeignKey, PrimaryKey, SqlSchema, Table};

type AssertionResult<T> = Result<T, anyhow::Error>;

pub trait SqlSchemaExt {
    fn assert_table<'a>(&'a self, table_name: &str) -> AssertionResult<TableAssertion<'a>>;
}

impl SqlSchemaExt for SqlSchema {
    fn assert_table<'a>(&'a self, table_name: &str) -> AssertionResult<TableAssertion<'a>> {
        self.table(table_name).map(TableAssertion).map_err(|_| {
            anyhow::anyhow!(
                "assert_table failed. Table {} not found. Tables in database: {:?}",
                table_name,
                self.tables.iter().map(|table| &table.name).collect::<Vec<_>>()
            )
        })
    }
}

pub struct TableAssertion<'a>(&'a Table);

impl<'a> TableAssertion<'a> {
    pub fn assert_foreign_keys_count(self, n: usize) -> AssertionResult<Self> {
        let fk_count = self.0.foreign_keys.len();
        anyhow::ensure!(
            fk_count == n,
            anyhow::anyhow!("Expected {} foreign keys, found {}.", n, fk_count)
        );

        Ok(self)
    }

    pub fn assert_fk_on_columns<F>(self, columns: &[&str], fk_assertions: F) -> AssertionResult<Self>
    where
        F: FnOnce(ForeignKeyAssertion<'a>) -> AssertionResult<ForeignKeyAssertion<'a>>,
    {
        if let Some(fk) = self.0.foreign_keys.iter().find(|fk| fk.columns == columns) {
            fk_assertions(ForeignKeyAssertion(fk))?;
        } else {
            anyhow::bail!("Could not find foreign key on {}.{:?}", self.0.name, columns);
        }

        Ok(self)
    }

    pub fn assert_has_column(self, column_name: &str) -> AssertionResult<Self> {
        self.0
            .column(column_name)
            .ok_or_else(|| anyhow::anyhow!("Column {} not found.", column_name))?;

        Ok(self)
    }

    pub fn assert_column<F>(self, column_name: &str, column_assertions: F) -> AssertionResult<Self>
    where
        F: FnOnce(ColumnAssertion<'a>) -> AssertionResult<ColumnAssertion<'a>>,
    {
        let column = self
            .0
            .column(column_name)
            .ok_or_else(|| anyhow::anyhow!("Column {} not found", column_name))?;

        column_assertions(ColumnAssertion(column))?;

        Ok(self)
    }

    pub fn assert_pk<F>(self, pk_assertions: F) -> AssertionResult<Self>
    where
        F: FnOnce(PrimaryKeyAssertion<'a>) -> AssertionResult<PrimaryKeyAssertion<'a>>,
    {
        let pk = self
            .0
            .primary_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Primary key not found on {}.", self.0.name))?;

        pk_assertions(PrimaryKeyAssertion(pk))?;

        Ok(self)
    }
}

pub struct ColumnAssertion<'a>(&'a Column);

impl<'a> ColumnAssertion<'a> {
    pub fn assert_default(self, expected: Option<&str>) -> AssertionResult<Self> {
        assert_eq!(self.0.default.as_ref().map(String::as_str), expected);

        Ok(self)
    }
}

pub struct PrimaryKeyAssertion<'a>(&'a PrimaryKey);

impl<'a> PrimaryKeyAssertion<'a> {
    pub fn assert_columns(self, column_names: &[&str]) -> AssertionResult<Self> {
        assert_eq!(self.0.columns, column_names);

        Ok(self)
    }
}

pub struct ForeignKeyAssertion<'a>(&'a ForeignKey);

impl<'a> ForeignKeyAssertion<'a> {
    pub fn assert_references(self, table: &str, columns: &[&str]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.0.referenced_table == table && self.0.referenced_columns == columns,
            "assert_references failed"
        );

        Ok(self)
    }
}
