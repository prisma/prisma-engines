mod migration_assertions;
mod quaint_result_set_ext;

pub use migration_assertions::*;
pub use quaint_result_set_ext::*;

use datamodel_connector::Connector;
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use sql_schema_describer::{
    Column, ColumnTypeFamily, DefaultKind, DefaultValue, Enum, ForeignKey, ForeignKeyAction, Index, IndexType,
    PrimaryKey, SqlSchema, Table,
};
use test_setup::{BitFlags, Tags};

pub trait SqlSchemaExt {
    fn assert_table<'a>(&'a self, table_name: &str) -> TableAssertion<'a>;
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

    pub fn assert_equals(self, other: &SqlSchema) -> Self {
        assert_eq!(&self.schema, other);
        self
    }

    pub fn assert_ne(self, other: &SqlSchema) -> Self {
        assert_ne!(&self.schema, other);
        self
    }

    #[track_caller]
    fn find_table(&self, table_name: &str) -> &sql_schema_describer::Table {
        match self.schema.tables.iter().find(|t| {
            if self.tags.contains(Tags::LowerCasesTableNames) {
                t.name.eq_ignore_ascii_case(table_name)
            } else {
                t.name == table_name
            }
        }) {
            Some(table) => table,
            None => panic!(
                "assert_has_table failed. Table {} not found. Tables in database: {:?}",
                table_name,
                self.schema.tables.iter().map(|table| &table.name).collect::<Vec<_>>()
            ),
        }
    }

    #[track_caller]
    pub fn assert_has_table(self, table_name: &str) -> Self {
        self.find_table(table_name);
        self
    }

    #[track_caller]
    pub fn assert_table<F>(self, table_name: &str, table_assertions: F) -> Self
    where
        F: for<'a> FnOnce(TableAssertion<'a>) -> TableAssertion<'a>,
    {
        let table = self.find_table(table_name);
        table_assertions(TableAssertion::new(table, self.tags));
        self
    }

    pub fn assert_has_no_enum(self, enum_name: &str) -> Self {
        let has_matching_enum = self.schema.enums.iter().any(|enm| {
            if self.tags.contains(Tags::LowerCasesTableNames) {
                enm.name.eq_ignore_ascii_case(enum_name)
            } else {
                enm.name == enum_name
            }
        });

        if has_matching_enum {
            panic!("Expected no enum named {}, found one", enum_name);
        }

        self
    }

    pub fn assert_enum<F>(self, enum_name: &str, enum_assertions: F) -> Self
    where
        F: for<'a> FnOnce(EnumAssertion<'a>) -> EnumAssertion<'a>,
    {
        let r#enum = match self.schema.get_enum(enum_name) {
            Some(enm) => enm,
            None => panic!("Assertion failed. Enum `{}` not found", enum_name),
        };

        enum_assertions(EnumAssertion(r#enum));
        self
    }

    #[track_caller]
    pub fn assert_tables_count(self, expected_count: usize) -> Self {
        let actual_count = self.schema.tables.len();

        assert_eq!(
            actual_count, expected_count,
            "Assertion failed. Expected the schema to have {expected_count} tables, found {actual_count}. ({table_names:?})",
            expected_count = expected_count,
            actual_count = actual_count,
            table_names = self.schema.tables.iter().map(|t| t.name.as_str()).collect::<Vec<&str>>(),
        );

        self
    }

    pub fn debug_print(self) -> Self {
        println!("{:?}", &self.schema);

        self
    }
}

pub struct EnumAssertion<'a>(&'a Enum);

impl<'a> EnumAssertion<'a> {
    pub fn assert_values(self, expected_values: &[&'static str]) -> Self {
        assert!(
            self.0.values == expected_values,
            "Assertion failed. The `{}` enum does not contain the expected variants.\nExpected:\n{:#?}\n\nFound:\n{:#?}\n",
            self.0.name,
            expected_values,
            self.0.values,
        );
        self
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

    pub fn assert_column_count(self, n: usize) -> Self {
        let columns_count = self.table.columns.len();

        assert!(
            columns_count == n,
            "Assertion failed. Expected {n} columns, found {columns_count}. {columns:#?}",
            n = n,
            columns_count = columns_count,
            columns = &self.table.columns,
        );
        self
    }

    pub fn assert_foreign_keys_count(self, n: usize) -> Self {
        let fk_count = self.table.foreign_keys.len();
        assert!(fk_count == n, "Expected {} foreign keys, found {}.", n, fk_count);
        self
    }

    pub fn assert_has_fk(self, fk: &ForeignKey) -> Self {
        let matching_fk = self.table.foreign_keys.iter().any(|found| found == fk);
        assert!(matching_fk, "Assertion failed. Could not find fk.");
        self
    }

    pub fn assert_fk_on_columns<F>(self, columns: &[&str], fk_assertions: F) -> Self
    where
        F: FnOnce(ForeignKeyAssertion<'a>) -> ForeignKeyAssertion<'a>,
    {
        if let Some(fk) = self.table.foreign_keys.iter().find(|fk| fk.columns == columns) {
            fk_assertions(ForeignKeyAssertion::new(fk, self.tags));
        } else {
            panic!("Could not find foreign key on {}.{:?}", self.table.name, columns);
        }

        self
    }

    pub fn assert_does_not_have_column(self, column_name: &str) -> Self {
        if self.table.column(column_name).is_some() {
            panic!(
                "Assertion failed: found column `{}` on `{}`.",
                column_name, self.table.name
            );
        }
        self
    }

    #[track_caller]
    pub fn assert_has_column(self, column_name: &str) -> Self {
        match self.table.column(column_name) {
            Some(_) => self,
            None => panic!(
                "Assertion failed: column {} not found. Existing columns: {:?}",
                column_name,
                self.table.columns.iter().map(|col| &col.name).collect::<Vec<_>>()
            ),
        }
    }

    pub fn assert_column<F>(self, column_name: &str, column_assertions: F) -> Self
    where
        F: FnOnce(ColumnAssertion<'a>) -> ColumnAssertion<'a>,
    {
        let this = self.assert_has_column(column_name);
        let column = this.table.column(column_name).unwrap();

        column_assertions(ColumnAssertion::new(column, self.tags));
        this
    }

    pub fn assert_columns_count(self, count: usize) -> Self {
        let actual_count = self.table.columns.len();

        assert!(
            actual_count == count,
            "Assertion failed: expected {} columns, found {}",
            count,
            actual_count,
        );

        self
    }

    pub fn assert_has_no_pk(self) -> Self {
        assert!(
            self.table.primary_key.is_none(),
            "Assertion failed: expected no primary key on {}, but found one. ({:?})",
            self.table.name,
            self.table.primary_key
        );

        self
    }

    pub fn assert_pk<F>(self, pk_assertions: F) -> Self
    where
        F: FnOnce(PrimaryKeyAssertion<'a>) -> PrimaryKeyAssertion<'a>,
    {
        match self.table.primary_key.as_ref() {
            Some(pk) => {
                pk_assertions(PrimaryKeyAssertion { pk, table: self.table });
                self
            }
            None => panic!("Primary key not found on {}.", self.table.name),
        }
    }

    pub fn assert_indexes_count(self, n: usize) -> Self {
        let idx_count = self.table.indices.len();
        assert!(idx_count == n, "Expected {} indexes, found {}.", n, idx_count);
        self
    }

    pub fn assert_index_on_columns<F>(self, columns: &[&str], index_assertions: F) -> Self
    where
        F: FnOnce(IndexAssertion<'a>) -> IndexAssertion<'a>,
    {
        if let Some(idx) = self.table.indices.iter().find(|idx| idx.columns == columns) {
            index_assertions(IndexAssertion(idx));
        } else {
            panic!("Could not find index on {}.{:?}", self.table.name, columns);
        }

        self
    }

    pub fn debug_print(self) -> Self {
        println!("{:?}", self.table);
        self
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

    pub fn assert_auto_increments(self) -> Self {
        assert!(
            self.column.auto_increment,
            "Assertion failed. Expected column `{}` to be auto-incrementing.",
            self.column.name,
        );

        self
    }

    pub fn assert_no_auto_increment(self) -> Self {
        assert!(
            !self.column.auto_increment,
            "Assertion failed. Expected column `{}` not to be auto-incrementing.",
            self.column.name,
        );

        self
    }

    pub fn assert_default(self, expected: Option<DefaultValue>) -> Self {
        let found = &self.column.default.as_ref().map(|d| d.kind());

        assert!(
            found == &expected.as_ref().map(|d| d.kind()),
            "Assertion failed. Expected default: {:?}, but found {:?}",
            expected,
            found
        );

        self
    }

    pub fn assert_full_data_type(self, full_data_type: &str) -> Self {
        let found = &self.column.tpe.full_data_type;

        assert!(
            found == full_data_type,
            "Assertion failed: expected the full_data_type for the `{}` column to be `{}`, found `{}`",
            self.column.name,
            full_data_type,
            found
        );

        self
    }

    pub fn assert_has_no_default(self) -> Self {
        self.assert_default(None)
    }

    pub fn assert_int_default(self, expected: i64) -> Self {
        self.assert_default(Some(DefaultValue::value(expected)))
    }

    pub fn assert_default_value(self, expected: &prisma_value::PrismaValue) -> Self {
        let found = &self.column.default;

        match found.as_ref().map(|d| d.kind()) {
            Some(DefaultKind::Value(ref val)) => assert!(
                val == expected,
                "Assertion failed. Expected the default value for `{}` to be `{:?}`, got `{:?}`",
                self.column.name,
                expected,
                val
            ),
            other => panic!(
                "Assertion failed. Expected default: {:?}, but found {:?}",
                expected, other
            ),
        }

        self
    }

    pub fn assert_dbgenerated(self, expected: &str) -> Self {
        let found = &self.column.default;

        match found.as_ref().map(|d| d.kind()) {
            Some(DefaultKind::DbGenerated(val)) => assert!(
                val == expected,
                "Assertion failed. Expected the default value for `{}` to be dbgenerated with `{:?}`, got `{:?}`",
                self.column.name,
                expected,
                val
            ),
            other => panic!(
                "Assertion failed. Expected default: {:?}, but found {:?}",
                expected, other
            ),
        }

        self
    }

    pub fn assert_enum_default(self, expected: &str) -> Self {
        let default = self.column.default.as_ref().unwrap();

        assert!(matches!(default.kind(), DefaultKind::Value(PrismaValue::Enum(s)) if s == expected));

        self
    }

    pub fn assert_native_type(self, expected: &str, connector: &dyn Connector) -> Self {
        let found = connector.render_native_type(self.column.tpe.native_type.clone().unwrap());
        assert!(
            found == expected,
            "Assertion failed. Expected the column native type for `{}` to be `{:?}`, found `{:?}`",
            self.column.name,
            expected,
            found,
        );

        self
    }

    pub fn assert_type_family(self, expected: ColumnTypeFamily) -> Self {
        let found = &self.column.tpe.family;

        let expected = match expected {
            ColumnTypeFamily::Enum(tbl_name) if self.tags.contains(Tags::LowerCasesTableNames) => {
                ColumnTypeFamily::Enum(tbl_name.to_lowercase())
            }
            _ => expected,
        };

        assert!(
            found == &expected,
            "Assertion failed. Expected the column type family for `{}` to be `{:?}`, found `{:?}`",
            self.column.name,
            expected,
            found,
        );

        self
    }

    pub fn assert_type_is_bigint(self) -> Self {
        let found = &self.column.tpe.family;

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::BigInt,
            "Assertion failed. Expected a BigInt column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_bytes(self) -> Self {
        let found = &self.column.tpe.family;

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::Binary,
            "Assertion failed. Expected a bytes column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_decimal(self) -> Self {
        let found = &self.column.tpe.family;

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::Decimal,
            "Assertion failed. Expected a decimal column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_enum(self) -> Self {
        let found = &self.column.tpe.family;

        assert!(
            matches!(found, sql_schema_describer::ColumnTypeFamily::Enum(_)),
            "Assertion failed. Expected an enum column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_string(self) -> Self {
        let found = &self.column.tpe.family;

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::String,
            "Assertion failed. Expected a string column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_int(self) -> Self {
        let found = &self.column.tpe.family;

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::Int,
            "Assertion failed. Expected an integer column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_is_list(self) -> Self {
        assert!(
            self.column.tpe.arity.is_list(),
            "Assertion failed. Expected column `{}` to be a list, got {:?}",
            self.column.name,
            self.column.tpe.arity,
        );

        self
    }

    pub fn assert_is_nullable(self) -> Self {
        assert!(
            self.column.tpe.arity.is_nullable(),
            "Assertion failed. Expected column `{}` to be nullable, got {:?}",
            self.column.name,
            self.column.tpe.arity,
        );

        self
    }

    pub fn assert_is_required(self) -> Self {
        assert!(
            self.column.tpe.arity.is_required(),
            "Assertion failed. Expected column `{}` to be NOT NULL, got {:?}",
            self.column.name,
            self.column.tpe.arity,
        );

        self
    }
}

pub struct PrimaryKeyAssertion<'a> {
    pk: &'a PrimaryKey,
    table: &'a Table,
}

impl<'a> PrimaryKeyAssertion<'a> {
    pub fn assert_columns(self, column_names: &[&str]) -> Self {
        assert_eq!(self.pk.columns, column_names);

        self
    }

    pub fn assert_has_autoincrement(self) -> Self {
        assert!(
            self.table
                .columns
                .iter()
                .any(|column| self.pk.columns.contains(&column.name) && column.auto_increment),
            "Assertion failed: expected a sequence on the primary key, found none."
        );

        self
    }

    pub fn assert_has_no_autoincrement(self) -> Self {
        assert!(
            !self
                .table
                .columns
                .iter()
                .any(|column| self.pk.columns.contains(&column.name) && column.auto_increment),
            "Assertion failed: expected no sequence on the primary key, but found one."
        );

        self
    }

    pub fn debug_print(self) -> Self {
        println!("{:?}", &self.pk);
        self
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

    #[track_caller]
    pub fn assert_references(self, table: &str, columns: &[&str]) -> Self {
        assert!(
            self.is_same_table_name(&self.fk.referenced_table, table) && self.fk.referenced_columns == columns,
            r#"Assertion failed. Expected reference to "{}" ({:?}). Found "{}" ({:?}) "#,
            table,
            columns,
            self.fk.referenced_table,
            self.fk.referenced_columns,
        );

        self
    }

    #[track_caller]
    pub fn assert_referential_action_on_delete(self, action: ForeignKeyAction) -> Self {
        assert!(
            self.fk.on_delete_action == action,
            "Assertion failed: expected foreign key to {:?} on delete, but got {:?}.",
            action,
            self.fk.on_delete_action
        );

        self
    }

    #[track_caller]
    pub fn assert_referential_action_on_update(self, action: ForeignKeyAction) -> Self {
        assert!(
            self.fk.on_update_action == action,
            "Assertion failed: expected foreign key to {:?} on update, but got {:?}.",
            action,
            self.fk.on_update_action
        );

        self
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
    pub fn assert_name(self, name: &str) -> Self {
        assert_eq!(self.0.name, name);

        self
    }

    pub fn assert_is_unique(self) -> Self {
        assert_eq!(self.0.tpe, IndexType::Unique);

        self
    }

    pub fn assert_is_not_unique(self) -> Self {
        assert_eq!(self.0.tpe, IndexType::Normal);

        self
    }
}
