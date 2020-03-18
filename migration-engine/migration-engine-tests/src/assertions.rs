use pretty_assertions::assert_eq;
use sql_schema_describer::{
    Column, DefaultValue, Enum, ForeignKey, ForeignKeyAction, Index, IndexType, PrimaryKey,
    SqlSchema, Table,
};

pub(crate) type AssertionResult<T> = Result<T, anyhow::Error>;

pub trait SqlSchemaExt {
    fn assert_table<'a>(&'a self, table_name: &str) -> AssertionResult<TableAssertion<'a>>;
}

impl SqlSchemaExt for SqlSchema {
    fn assert_table<'a>(&'a self, table_name: &str) -> AssertionResult<TableAssertion<'a>> {
        self.table(table_name).map(TableAssertion).map_err(|_| {
            anyhow::anyhow!(
                "assert_table failed. Table {} not found. Tables in database: {:?}",
                table_name,
                self.tables
                    .iter()
                    .map(|table| &table.name)
                    .collect::<Vec<_>>()
            )
        })
    }
}

pub struct SchemaAssertion(pub SqlSchema);

impl SchemaAssertion {
    pub fn into_schema(self) -> SqlSchema {
        self.0
    }

    pub fn assert_equals(self, other: &SqlSchema) -> AssertionResult<Self> {
        assert_eq!(&self.0, other);

        Ok(self)
    }

    pub fn assert_ne(self, other: &SqlSchema) -> AssertionResult<Self> {
        assert_ne!(&self.0, other);

        Ok(self)
    }

    pub fn assert_table<F>(self, table_name: &str, table_assertions: F) -> AssertionResult<Self>
    where
        F: for<'a> FnOnce(TableAssertion<'a>) -> AssertionResult<TableAssertion<'a>>,
    {
        let table_result = self.0.table(table_name);
        let table = table_result.map(TableAssertion).map_err(|_| {
            anyhow::anyhow!(
                "assert_table failed. Table {} not found. Tables in database: {:?}",
                table_name,
                self.0
                    .tables
                    .iter()
                    .map(|table| &table.name)
                    .collect::<Vec<_>>()
            )
        })?;

        table_assertions(table)?;

        Ok(self)
    }

    pub fn assert_has_no_enum(self, enum_name: &str) -> AssertionResult<Self> {
        assert!(self.0.get_enum(enum_name).is_none());

        Ok(self)
    }

    pub fn assert_enum<F>(self, enum_name: &str, enum_assertions: F) -> AssertionResult<Self>
    where
        F: for<'a> FnOnce(EnumAssertion<'a>) -> AssertionResult<EnumAssertion<'a>>,
    {
        let r#enum = self
            .0
            .get_enum(enum_name)
            .ok_or_else(|| anyhow::anyhow!("Assertion failed. Enum `{}` not found", enum_name))?;

        enum_assertions(EnumAssertion(&r#enum))?;

        Ok(self)
    }

    pub fn debug_print(self) -> Self {
        dbg!(&self.0);

        self
    }
}

pub struct EnumAssertion<'a>(&'a Enum);

impl<'a> EnumAssertion<'a> {
    pub fn assert_values(self, expected_values: &[&'static str]) -> AssertionResult<Self> {
        dbg!(&self.0);
        assert_eq!(self.0.values, expected_values);

        Ok(self)
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

    pub fn assert_has_fk(self, fk: &ForeignKey) -> AssertionResult<Self> {
        let matching_fk = self.0.foreign_keys.iter().any(|found| found == fk);

        anyhow::ensure!(matching_fk, "Assertion failed. Could not find fk.");

        Ok(self)
    }

    pub fn assert_fk_on_columns<F>(
        self,
        columns: &[&str],
        fk_assertions: F,
    ) -> AssertionResult<Self>
    where
        F: FnOnce(ForeignKeyAssertion<'a>) -> AssertionResult<ForeignKeyAssertion<'a>>,
    {
        if let Some(fk) = self.0.foreign_keys.iter().find(|fk| fk.columns == columns) {
            fk_assertions(ForeignKeyAssertion(fk))?;
        } else {
            anyhow::bail!(
                "Could not find foreign key on {}.{:?}",
                self.0.name,
                columns
            );
        }

        Ok(self)
    }

    pub fn assert_does_not_have_column(self, column_name: &str) -> AssertionResult<Self> {
        if self.0.column(column_name).is_some() {
            anyhow::bail!(
                "Assertion failed: found column `{}` on `{}`.",
                column_name,
                self.0.name
            );
        }

        Ok(self)
    }

    pub fn assert_has_column(self, column_name: &str) -> AssertionResult<Self> {
        self.0.column(column_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Assertion failed: column {} not found. Existing columns: {:?}",
                column_name,
                self.0
                    .columns
                    .iter()
                    .map(|col| &col.name)
                    .collect::<Vec<_>>()
            )
        })?;

        Ok(self)
    }

    pub fn assert_column<F>(self, column_name: &str, column_assertions: F) -> AssertionResult<Self>
    where
        F: FnOnce(ColumnAssertion<'a>) -> AssertionResult<ColumnAssertion<'a>>,
    {
        let this = self.assert_has_column(column_name)?;
        let column = this.0.column(column_name).unwrap();

        column_assertions(ColumnAssertion(column))?;

        Ok(this)
    }

    pub fn assert_columns_count(self, count: usize) -> AssertionResult<Self> {
        let actual_count = self.0.columns.len();

        anyhow::ensure!(
            actual_count == count,
            "Assertion failed: expected {} columns, found {}",
            count,
            actual_count,
        );

        Ok(self)
    }

    pub fn assert_has_no_pk(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.0.primary_key.is_none(),
            "Assertion failed: expected no primary key on {}, but found one. ({:?})",
            self.0.name,
            self.0.primary_key
        );

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

    pub fn assert_indexes_count(self, n: usize) -> AssertionResult<Self> {
        let idx_count = self.0.indices.len();
        anyhow::ensure!(
            idx_count == n,
            anyhow::anyhow!("Expected {} indexes, found {}.", n, idx_count)
        );

        Ok(self)
    }

    pub fn assert_index_on_columns<F>(
        self,
        columns: &[&str],
        index_assertions: F,
    ) -> AssertionResult<Self>
    where
        F: FnOnce(IndexAssertion<'a>) -> AssertionResult<IndexAssertion<'a>>,
    {
        if let Some(idx) = self.0.indices.iter().find(|idx| idx.columns == columns) {
            index_assertions(IndexAssertion(idx))?;
        } else {
            anyhow::bail!("Could not find index on {}.{:?}", self.0.name, columns);
        }

        Ok(self)
    }
}

pub struct ColumnAssertion<'a>(&'a Column);

impl<'a> ColumnAssertion<'a> {
    pub fn assert_default(self, expected: Option<&str>) -> AssertionResult<Self> {
        let found = self
            .0
            .default
            .as_ref()
            .map(|default_value| match default_value {
                DefaultValue::VALUE(s) => s,
                DefaultValue::SEQUENCE(s) => s,
                DefaultValue::DBGENERATED(s) => s,
                DefaultValue::NOW => "CURRENT_TIMESTAMP",
            });

        anyhow::ensure!(
            found == expected,
            "Assertion failed. Expected default: {:?}, but found {:?}",
            expected,
            found
        );

        Ok(self)
    }

    pub fn assert_type_is_string(self) -> AssertionResult<Self> {
        let found = &self.0.tpe.family;

        anyhow::ensure!(
            found == &sql_schema_describer::ColumnTypeFamily::String,
            "Assertion failed. Expected a string column, got {:?}.",
            found
        );

        Ok(self)
    }

    pub fn assert_type_is_int(self) -> AssertionResult<Self> {
        let found = &self.0.tpe.family;

        anyhow::ensure!(
            found == &sql_schema_describer::ColumnTypeFamily::Int,
            "Assertion failed. Expected an integer column, got {:?}.",
            found
        );

        Ok(self)
    }

    pub fn assert_is_required(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.0.tpe.arity.is_required(),
            "Assertion failed. Expected column `{}` to be NOT NULL, got {:?}",
            self.0.name,
            self.0.tpe.arity,
        );

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
            r#"Assertion failed. Expected reference to "{}" ({:?}). Found "{}" ({:?}) "#,
            table,
            columns,
            self.0.referenced_table,
            self.0.referenced_columns,
        );

        Ok(self)
    }

    pub fn assert_cascades_on_delete(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.0.on_delete_action == ForeignKeyAction::Cascade,
            "Assertion failed: expected foreign key to cascade on delete."
        );

        Ok(self)
    }
}

pub struct IndexAssertion<'a>(&'a Index);

impl<'a> IndexAssertion<'a> {
    pub fn assert_name(self, name: &str) -> AssertionResult<Self> {
        assert_eq!(self.0.name, name);

        Ok(self)
    }

    pub fn assert_is_unique(self) -> AssertionResult<Self> {
        assert_eq!(self.0.tpe, IndexType::Unique);

        Ok(self)
    }

    pub fn assert_is_not_unique(self) -> AssertionResult<Self> {
        assert_eq!(self.0.tpe, IndexType::Normal);

        Ok(self)
    }
}
