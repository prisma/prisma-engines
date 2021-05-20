use datamodel_connector::Connector;
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use sql_schema_describer::{
    Column, ColumnTypeFamily, DefaultKind, DefaultValue, Enum, ForeignKey, ForeignKeyAction, Index, IndexType,
    PrimaryKey, SqlSchema, Table,
};
use test_setup::{BitFlags, Tags};

pub(crate) type AssertionResult<T> = Result<T, anyhow::Error>;

pub trait SqlSchemaExt {
    fn assert_table<'a>(&'a self, table_name: &str) -> AssertionResult<TableAssertion<'a>>;
}

pub struct SchemaAssertion {
    schema: SqlSchema,
    tags: BitFlags<Tags>,
}

impl SchemaAssertion {
    pub fn new(schema: SqlSchema, tags: BitFlags<Tags>) -> Self {
        Self { schema, tags }
    }

    pub fn into_schema(self) -> SqlSchema {
        self.schema
    }

    pub fn assert_equals(self, other: &SqlSchema) -> AssertionResult<Self> {
        assert_eq!(&self.schema, other);

        Ok(self)
    }

    pub fn assert_ne(self, other: &SqlSchema) -> AssertionResult<Self> {
        assert_ne!(&self.schema, other);

        Ok(self)
    }

    fn find_table(&self, table_name: &str) -> anyhow::Result<&sql_schema_describer::Table> {
        match self.schema.tables.iter().find(|t| {
            if self.tags.contains(Tags::LowerCasesTableNames) {
                t.name.eq_ignore_ascii_case(table_name)
            } else {
                t.name == table_name
            }
        }) {
            Some(table) => Ok(table),
            None => Err(anyhow::anyhow!(
                "assert_has_table failed. Table {} not found. Tables in database: {:?}",
                table_name,
                self.schema.tables.iter().map(|table| &table.name).collect::<Vec<_>>()
            )),
        }
    }

    pub fn assert_has_table(self, table_name: &str) -> AssertionResult<Self> {
        self.find_table(table_name)?;
        Ok(self)
    }

    pub fn assert_table<F>(self, table_name: &str, table_assertions: F) -> AssertionResult<Self>
    where
        F: for<'a> FnOnce(TableAssertion<'a>) -> AssertionResult<TableAssertion<'a>>,
    {
        let table = self.find_table(table_name)?;

        table_assertions(TableAssertion::new(table, self.tags))?;

        Ok(self)
    }

    #[track_caller]
    pub fn assert_table_bang<F>(self, table_name: &str, table_assertions: F) -> Self
    where
        F: for<'a> FnOnce(TableAssertion<'a>) -> AssertionResult<TableAssertion<'a>>,
    {
        let table = self.find_table(table_name).unwrap();

        table_assertions(TableAssertion::new(table, self.tags)).unwrap();

        self
    }

    pub fn assert_has_no_enum(self, enum_name: &str) -> AssertionResult<Self> {
        let has_matching_enum = self.schema.enums.iter().any(|enm| {
            if self.tags.contains(Tags::LowerCasesTableNames) {
                enm.name.eq_ignore_ascii_case(enum_name)
            } else {
                enm.name == enum_name
            }
        });

        if has_matching_enum {
            anyhow::bail!("Expected no enum named {}, found one", enum_name);
        }

        Ok(self)
    }

    pub fn assert_enum<F>(self, enum_name: &str, enum_assertions: F) -> AssertionResult<Self>
    where
        F: for<'a> FnOnce(EnumAssertion<'a>) -> AssertionResult<EnumAssertion<'a>>,
    {
        let r#enum = self
            .schema
            .get_enum(enum_name)
            .ok_or_else(|| anyhow::anyhow!("Assertion failed. Enum `{}` not found", enum_name))?;

        enum_assertions(EnumAssertion(&r#enum))?;

        Ok(self)
    }

    pub fn assert_tables_count(self, expected_count: usize) -> AssertionResult<Self> {
        let actual_count = self.schema.tables.len();

        anyhow::ensure!(
            actual_count == expected_count,
            "Assertion failed. Expected the schema to have {expected_count} tables, found {actual_count}. ({table_names:?})",
            expected_count = expected_count,
            actual_count = actual_count,
            table_names = self.schema.tables.iter().map(|t| t.name.as_str()).collect::<Vec<&str>>(),
        );

        Ok(self)
    }

    pub fn debug_print(self) -> Self {
        println!("{:?}", &self.schema);

        self
    }
}

pub struct EnumAssertion<'a>(&'a Enum);

impl<'a> EnumAssertion<'a> {
    pub fn assert_values(self, expected_values: &[&'static str]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.0.values == expected_values,
            "Assertion failed. The `{}` enum does not contain the expected variants.\nExpected:\n{:#?}\n\nFound:\n{:#?}\n",
            self.0.name,
            expected_values,
            self.0.values,
        );

        Ok(self)
    }
}

#[derive(Clone, Copy)]
pub struct TableAssertion<'a> {
    table: &'a Table,
    tags: BitFlags<Tags>,
}

impl<'a> TableAssertion<'a> {
    pub fn new(table: &'a Table, tags: BitFlags<Tags>) -> Self {
        Self { table, tags }
    }

    pub fn assert_column_count(self, n: usize) -> AssertionResult<Self> {
        let columns_count = self.table.columns.len();

        anyhow::ensure!(
            columns_count == n,
            anyhow::anyhow!(
                "Assertion failed. Expected {n} columns, found {columns_count}. {columns:#?}",
                n = n,
                columns_count = columns_count,
                columns = &self.table.columns,
            )
        );

        Ok(self)
    }

    pub fn assert_foreign_keys_count(self, n: usize) -> AssertionResult<Self> {
        let fk_count = self.table.foreign_keys.len();
        anyhow::ensure!(
            fk_count == n,
            anyhow::anyhow!("Expected {} foreign keys, found {}.", n, fk_count)
        );

        Ok(self)
    }

    pub fn assert_has_fk(self, fk: &ForeignKey) -> AssertionResult<Self> {
        let matching_fk = self.table.foreign_keys.iter().any(|found| found == fk);

        anyhow::ensure!(matching_fk, "Assertion failed. Could not find fk.");

        Ok(self)
    }

    pub fn assert_fk_on_columns<F>(self, columns: &[&str], fk_assertions: F) -> AssertionResult<Self>
    where
        F: FnOnce(ForeignKeyAssertion<'a>) -> AssertionResult<ForeignKeyAssertion<'a>>,
    {
        if let Some(fk) = self.table.foreign_keys.iter().find(|fk| fk.columns == columns) {
            fk_assertions(ForeignKeyAssertion::new(fk, self.tags))?;
        } else {
            anyhow::bail!("Could not find foreign key on {}.{:?}", self.table.name, columns);
        }

        Ok(self)
    }

    pub fn assert_does_not_have_column(self, column_name: &str) -> AssertionResult<Self> {
        if self.table.column(column_name).is_some() {
            anyhow::bail!(
                "Assertion failed: found column `{}` on `{}`.",
                column_name,
                self.table.name
            );
        }

        Ok(self)
    }

    pub fn assert_has_column(self, column_name: &str) -> AssertionResult<Self> {
        self.table.column(column_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Assertion failed: column {} not found. Existing columns: {:?}",
                column_name,
                self.table.columns.iter().map(|col| &col.name).collect::<Vec<_>>()
            )
        })?;

        Ok(self)
    }

    pub fn assert_column<F>(self, column_name: &str, column_assertions: F) -> AssertionResult<Self>
    where
        F: FnOnce(ColumnAssertion<'a>) -> AssertionResult<ColumnAssertion<'a>>,
    {
        let this = self.assert_has_column(column_name)?;
        let column = this.table.column(column_name).unwrap();

        column_assertions(ColumnAssertion::new(column, self.tags))?;

        Ok(this)
    }

    pub fn assert_columns_count(self, count: usize) -> AssertionResult<Self> {
        let actual_count = self.table.columns.len();

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
            self.table.primary_key.is_none(),
            "Assertion failed: expected no primary key on {}, but found one. ({:?})",
            self.table.name,
            self.table.primary_key
        );

        Ok(self)
    }

    pub fn assert_pk<F>(self, pk_assertions: F) -> AssertionResult<Self>
    where
        F: FnOnce(PrimaryKeyAssertion<'a>) -> AssertionResult<PrimaryKeyAssertion<'a>>,
    {
        let pk = self
            .table
            .primary_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Primary key not found on {}.", self.table.name))?;

        pk_assertions(PrimaryKeyAssertion { pk, table: self.table })?;

        Ok(self)
    }

    pub fn assert_indexes_count(self, n: usize) -> AssertionResult<Self> {
        let idx_count = self.table.indices.len();
        anyhow::ensure!(
            idx_count == n,
            anyhow::anyhow!("Expected {} indexes, found {}.", n, idx_count)
        );

        Ok(self)
    }

    pub fn assert_index_on_columns<F>(self, columns: &[&str], index_assertions: F) -> AssertionResult<Self>
    where
        F: FnOnce(IndexAssertion<'a>) -> AssertionResult<IndexAssertion<'a>>,
    {
        if let Some(idx) = self.table.indices.iter().find(|idx| idx.columns == columns) {
            index_assertions(IndexAssertion(idx))?;
        } else {
            anyhow::bail!("Could not find index on {}.{:?}", self.table.name, columns);
        }

        Ok(self)
    }

    pub fn debug_print(self) -> AssertionResult<Self> {
        println!("{:?}", self.table);
        Ok(self)
    }
}

pub struct ColumnAssertion<'a> {
    column: &'a Column,
    tags: BitFlags<Tags>,
}

impl<'a> ColumnAssertion<'a> {
    pub fn new(column: &'a Column, tags: BitFlags<Tags>) -> Self {
        Self { column, tags }
    }

    pub fn assert_auto_increments(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.column.auto_increment,
            "Assertion failed. Expected column `{}` to be auto-incrementing.",
            self.column.name,
        );

        Ok(self)
    }

    pub fn assert_no_auto_increment(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            !self.column.auto_increment,
            "Assertion failed. Expected column `{}` not to be auto-incrementing.",
            self.column.name,
        );

        Ok(self)
    }

    pub fn assert_default(self, expected: Option<DefaultValue>) -> AssertionResult<Self> {
        let found = &self.column.default.as_ref().map(|d| d.kind());

        anyhow::ensure!(
            found == &expected.as_ref().map(|d| d.kind()),
            "Assertion failed. Expected default: {:?}, but found {:?}",
            expected,
            found
        );

        Ok(self)
    }

    pub fn assert_full_data_type(self, full_data_type: &str) -> AssertionResult<Self> {
        let found = &self.column.tpe.full_data_type;

        anyhow::ensure!(
            found == full_data_type,
            "Assertion failed: expected the full_data_type for the `{}` column to be `{}`, found `{}`",
            self.column.name,
            full_data_type,
            found
        );

        Ok(self)
    }

    pub fn assert_has_no_default(self) -> AssertionResult<Self> {
        self.assert_default(None)
    }

    pub fn assert_default_value(self, expected: &prisma_value::PrismaValue) -> AssertionResult<Self> {
        let found = &self.column.default;

        match found.as_ref().map(|d| d.kind()) {
            Some(DefaultKind::Value(ref val)) => anyhow::ensure!(
                val == expected,
                "Assertion failed. Expected the default value for `{}` to be `{:?}`, got `{:?}`",
                self.column.name,
                expected,
                val
            ),
            other => anyhow::bail!(
                "Assertion failed. Expected default: {:?}, but found {:?}",
                expected,
                other
            ),
        }

        Ok(self)
    }

    pub fn assert_dbgenerated(self, expected: &str) -> AssertionResult<Self> {
        let found = &self.column.default;

        match found.as_ref().map(|d| d.kind()) {
            Some(DefaultKind::DbGenerated(val)) => anyhow::ensure!(
                val == expected,
                "Assertion failed. Expected the default value for `{}` to be dbgenerated with `{:?}`, got `{:?}`",
                self.column.name,
                expected,
                val
            ),
            other => anyhow::bail!(
                "Assertion failed. Expected default: {:?}, but found {:?}",
                expected,
                other
            ),
        }

        Ok(self)
    }

    pub fn assert_enum_default(self, expected: &str) -> Self {
        let default = self.column.default.as_ref().unwrap();

        assert!(matches!(default.kind(), DefaultKind::Value(PrismaValue::Enum(s)) if s == expected));

        self
    }

    pub fn assert_native_type(self, expected: &str, connector: &dyn Connector) -> AssertionResult<Self> {
        let found = connector.render_native_type(self.column.tpe.native_type.clone().unwrap());
        anyhow::ensure!(
            found == expected,
            "Assertion failed. Expected the column native type for `{}` to be `{:?}`, found `{:?}`",
            self.column.name,
            expected,
            found,
        );

        Ok(self)
    }

    pub fn assert_type_family(self, expected: ColumnTypeFamily) -> AssertionResult<Self> {
        let found = &self.column.tpe.family;

        let expected = match expected {
            ColumnTypeFamily::Enum(tbl_name) if self.tags.contains(Tags::LowerCasesTableNames) => {
                ColumnTypeFamily::Enum(tbl_name.to_lowercase())
            }
            _ => expected,
        };

        anyhow::ensure!(
            found == &expected,
            "Assertion failed. Expected the column type family for `{}` to be `{:?}`, found `{:?}`",
            self.column.name,
            expected,
            found,
        );

        Ok(self)
    }

    pub fn assert_type_is_bigint(self) -> AssertionResult<Self> {
        let found = &self.column.tpe.family;

        anyhow::ensure!(
            found == &sql_schema_describer::ColumnTypeFamily::BigInt,
            "Assertion failed. Expected a BigInt column, got {:?}.",
            found
        );

        Ok(self)
    }

    pub fn assert_type_is_bytes(self) -> AssertionResult<Self> {
        let found = &self.column.tpe.family;

        anyhow::ensure!(
            found == &sql_schema_describer::ColumnTypeFamily::Binary,
            "Assertion failed. Expected a bytes column, got {:?}.",
            found
        );

        Ok(self)
    }

    pub fn assert_type_is_decimal(self) -> AssertionResult<Self> {
        let found = &self.column.tpe.family;

        anyhow::ensure!(
            found == &sql_schema_describer::ColumnTypeFamily::Decimal,
            "Assertion failed. Expected a decimal column, got {:?}.",
            found
        );

        Ok(self)
    }

    pub fn assert_type_is_enum(self) -> AssertionResult<Self> {
        let found = &self.column.tpe.family;

        assert!(
            matches!(found, sql_schema_describer::ColumnTypeFamily::Enum(_)),
            "Assertion failed. Expected an enum column, got {:?}.",
            found
        );

        Ok(self)
    }

    pub fn assert_type_is_string(self) -> AssertionResult<Self> {
        let found = &self.column.tpe.family;

        anyhow::ensure!(
            found == &sql_schema_describer::ColumnTypeFamily::String,
            "Assertion failed. Expected a string column, got {:?}.",
            found
        );

        Ok(self)
    }

    pub fn assert_type_is_int(self) -> AssertionResult<Self> {
        let found = &self.column.tpe.family;

        anyhow::ensure!(
            found == &sql_schema_describer::ColumnTypeFamily::Int,
            "Assertion failed. Expected an integer column, got {:?}.",
            found
        );

        Ok(self)
    }

    pub fn assert_is_list(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.column.tpe.arity.is_list(),
            "Assertion failed. Expected column `{}` to be a list, got {:?}",
            self.column.name,
            self.column.tpe.arity,
        );

        Ok(self)
    }

    pub fn assert_is_nullable(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.column.tpe.arity.is_nullable(),
            "Assertion failed. Expected column `{}` to be nullable, got {:?}",
            self.column.name,
            self.column.tpe.arity,
        );

        Ok(self)
    }

    pub fn assert_is_required(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.column.tpe.arity.is_required(),
            "Assertion failed. Expected column `{}` to be NOT NULL, got {:?}",
            self.column.name,
            self.column.tpe.arity,
        );

        Ok(self)
    }
}

pub struct PrimaryKeyAssertion<'a> {
    pk: &'a PrimaryKey,
    table: &'a Table,
}

impl<'a> PrimaryKeyAssertion<'a> {
    pub fn assert_columns(self, column_names: &[&str]) -> AssertionResult<Self> {
        assert_eq!(self.pk.columns, column_names);

        Ok(self)
    }

    pub fn assert_has_autoincrement(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.table
                .columns
                .iter()
                .any(|column| self.pk.columns.contains(&column.name) && column.auto_increment),
            "Assertion failed: expected a sequence on the primary key, found none."
        );

        Ok(self)
    }

    pub fn assert_has_no_autoincrement(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            !self
                .table
                .columns
                .iter()
                .any(|column| self.pk.columns.contains(&column.name) && column.auto_increment),
            "Assertion failed: expected no sequence on the primary key, but found one."
        );

        Ok(self)
    }

    pub fn debug_print(self) -> AssertionResult<Self> {
        println!("{:?}", &self.pk);
        Ok(self)
    }
}

pub struct ForeignKeyAssertion<'a> {
    fk: &'a ForeignKey,
    tags: BitFlags<Tags>,
}

impl<'a> ForeignKeyAssertion<'a> {
    pub fn new(fk: &'a ForeignKey, tags: BitFlags<Tags>) -> Self {
        Self { fk, tags }
    }

    pub fn assert_references(self, table: &str, columns: &[&str]) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.is_same_table_name(&self.fk.referenced_table, table) && self.fk.referenced_columns == columns,
            r#"Assertion failed. Expected reference to "{}" ({:?}). Found "{}" ({:?}) "#,
            table,
            columns,
            self.fk.referenced_table,
            self.fk.referenced_columns,
        );

        Ok(self)
    }

    pub fn assert_cascades_on_delete(self) -> AssertionResult<Self> {
        anyhow::ensure!(
            self.fk.on_delete_action == ForeignKeyAction::Cascade,
            "Assertion failed: expected foreign key to cascade on delete."
        );

        Ok(self)
    }

    fn is_same_table_name(&self, fst: &str, snd: &str) -> bool {
        if self.tags.contains(Tags::LowerCasesTableNames) {
            fst.eq_ignore_ascii_case(snd)
        } else {
            fst == snd
        }
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
