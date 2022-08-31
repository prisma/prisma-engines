mod migration_assertions;
mod quaint_result_set_ext;

pub use migration_assertions::*;
pub use quaint_result_set_ext::*;

use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use psl::datamodel_connector::Connector;
use sql::{
    postgres::PostgresSchemaExt,
    walkers::{ColumnWalker, ForeignKeyWalker, IndexWalker, TableWalker},
};
use sql_schema_describer::{
    self as sql,
    postgres::{SQLOperatorClassKind, SqlIndexAlgorithm},
    ColumnTypeFamily, DefaultKind, DefaultValue, Enum, ForeignKeyAction, IndexType, SQLSortOrder, SqlSchema,
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

    #[track_caller]
    fn find_table<'a>(&'a self, table_name: &str) -> TableWalker<'a> {
        match self.schema.table_walkers().find(|t| {
            if self.tags.contains(Tags::LowerCasesTableNames) {
                t.name().eq_ignore_ascii_case(table_name)
            } else {
                t.name() == table_name
            }
        }) {
            Some(table) => table,
            None => panic!(
                "assert_has_table failed. Table {} not found. Tables in database: {:?}",
                table_name,
                self.schema
                    .table_walkers()
                    .map(|table| table.name())
                    .collect::<Vec<_>>()
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
        table_assertions(TableAssertion { table, tags: self.tags });
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
        let actual_count = self.schema.tables_count();

        assert_eq!(
            actual_count, expected_count,
            "Assertion failed. Expected the schema to have {expected_count} tables, found {actual_count}. ({table_names:?})",
            expected_count = expected_count,
            actual_count = actual_count,
            table_names = self.schema.table_walkers().map(|t| t.name()).collect::<Vec<&str>>(),
        );

        self
    }

    #[track_caller]
    pub fn assert_views_count(self, expected_count: usize) -> Self {
        let actual_count = self.schema.view_walkers().count();

        assert_eq!(
            actual_count, expected_count,
            "Assertion failed. Expected the schema to have {expected_count} views, found {actual_count}. ({table_names:?})",
            expected_count = expected_count,
            actual_count = actual_count,
            table_names = self.schema.view_walkers().map(|t| t.name()).collect::<Vec<&str>>(),
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
    table: TableWalker<'a>,
    tags: BitFlags<Tags>,
}

impl<'a> TableAssertion<'a> {
    pub fn assert_column_count(self, n: usize) -> Self {
        let columns_count = self.table.columns().count();

        assert!(
            columns_count == n,
            "Assertion failed. Expected {n} columns, found {columns_count}.",
            n = n,
            columns_count = columns_count,
        );
        self
    }

    pub fn assert_foreign_keys_count(self, n: usize) -> Self {
        let fk_count = self.table.foreign_key_count();
        assert!(fk_count == n, "Expected {} foreign keys, found {}.", n, fk_count);
        self
    }

    #[track_caller]
    pub fn assert_fk_on_columns<F>(self, columns: &[&str], fk_assertions: F) -> Self
    where
        F: FnOnce(ForeignKeyAssertion<'a>) -> ForeignKeyAssertion<'a>,
    {
        if let Some(fk) = self
            .table
            .foreign_keys()
            .find(|fk| fk.constrained_columns().map(|c| c.name()).collect::<Vec<_>>() == columns)
        {
            fk_assertions(ForeignKeyAssertion { fk, tags: self.tags });
        } else {
            panic!("Could not find foreign key on {}.{:?}", self.table.name(), columns);
        }

        self
    }

    pub fn assert_fk_with_name(self, name: &str) -> Self {
        let matching_fk = self
            .table
            .foreign_keys()
            .any(|found| found.constraint_name() == Some(name));
        assert!(matching_fk, "Assertion failed. Could not find fk with name.");
        self
    }

    pub fn assert_does_not_have_column(self, column_name: &str) -> Self {
        if self.table.column(column_name).is_some() {
            panic!(
                "Assertion failed: found column `{}` on `{}`.",
                column_name,
                self.table.name()
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
                self.table.columns().map(|col| col.name()).collect::<Vec<_>>()
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
        let actual_count = self.table.columns().count();

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
            self.table.primary_key().is_none(),
            "Assertion failed: expected no primary key on {}, but found one.",
            self.table.name(),
        );

        self
    }

    pub fn assert_pk<F>(self, pk_assertions: F) -> Self
    where
        F: FnOnce(PrimaryKeyAssertion<'a>) -> PrimaryKeyAssertion<'a>,
    {
        match self.table.primary_key() {
            Some(pk) => {
                pk_assertions(PrimaryKeyAssertion { pk, tags: self.tags });
                self
            }
            None => panic!("Primary key not found on {}.", self.table.name()),
        }
    }

    #[track_caller]
    pub fn assert_indexes_count(self, n: usize) -> Self {
        let idx_count = self.table.indexes().filter(|idx| !idx.is_primary_key()).count();
        assert!(idx_count == n, "Expected {} indexes, found {}.", n, idx_count);
        self
    }

    pub fn assert_index_on_columns<F>(self, columns: &[&str], index_assertions: F) -> Self
    where
        F: FnOnce(IndexAssertion<'a>) -> IndexAssertion<'a>,
    {
        if let Some(idx) = self
            .table
            .indexes()
            .filter(|idx| !idx.is_primary_key())
            .find(|idx| idx.column_names().collect::<Vec<_>>() == columns)
        {
            index_assertions(IndexAssertion {
                index: idx,
                tags: self.tags,
            });
        } else {
            panic!("Could not find index on {}.{:?}", self.table.name(), columns);
        }

        self
    }

    pub fn assert_has_index_name_and_type(self, name: &str, unique: bool) -> Self {
        if self
            .table
            .indexes()
            .any(|idx| idx.name() == name && idx.is_unique() == unique)
        {
            self
        } else {
            panic!("Could not find index with name {} and correct type", name);
        }
    }
}

pub struct ColumnAssertion<'a> {
    column: ColumnWalker<'a>,
    tags: BitFlags<Tags>,
}

impl<'a> ColumnAssertion<'a> {
    pub fn new(column: ColumnWalker<'a>, tags: BitFlags<Tags>) -> Self {
        Self { column, tags }
    }

    pub fn assert_auto_increments(self) -> Self {
        assert!(
            self.column.is_autoincrement(),
            "Assertion failed. Expected column `{}` to be auto-incrementing.",
            self.column.name(),
        );

        self
    }

    pub fn assert_no_auto_increment(self) -> Self {
        assert!(
            !self.column.is_autoincrement(),
            "Assertion failed. Expected column `{}` not to be auto-incrementing.",
            self.column.name(),
        );

        self
    }

    #[track_caller]
    pub fn assert_default_kind(self, expected: Option<DefaultKind>) -> Self {
        let found = &self.column.default().map(|d| d.kind());

        assert!(
            self.column.default().map(|d| d.kind()) == expected.as_ref(),
            "Assertion failed. Expected default: {:?}, but found {:?}",
            expected,
            found
        );

        self
    }

    #[track_caller]
    pub fn assert_default(self, expected: Option<DefaultValue>) -> Self {
        let this = self.assert_default_kind(expected.clone().map(|val| val.into_kind()));
        let found = this.column.default().map(|d| d.constraint_name());
        let expected = expected.as_ref().map(|d| d.constraint_name());

        assert!(
            found == expected,
            "Assertion failed. Expected default constraint name: {:?}, but found {:?}",
            expected,
            found
        );

        this
    }

    pub fn assert_full_data_type(self, full_data_type: &str) -> Self {
        let found = &self.column.column_type().full_data_type;

        assert!(
            found == full_data_type,
            "Assertion failed: expected the full_data_type for the `{}` column to be `{}`, found `{}`",
            self.column.name(),
            full_data_type,
            found
        );

        self
    }

    pub fn assert_has_no_default(self) -> Self {
        self.assert_default(None)
    }

    pub fn assert_int_default(self, expected: i64) -> Self {
        self.assert_default_kind(Some(DefaultKind::Value(expected.into())))
    }

    pub fn assert_default_value(self, expected: &prisma_value::PrismaValue) -> Self {
        let found = self.column.default();

        match found.as_ref().map(|d| d.kind()) {
            Some(DefaultKind::Value(ref val)) => assert!(
                val == expected,
                "Assertion failed. Expected the default value for `{}` to be `{:?}`, got `{:?}`",
                self.column.name(),
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
        let found = self.column.default();

        match found.map(|d| d.kind()) {
            Some(DefaultKind::DbGenerated(Some(val))) => assert!(
                val == expected,
                "Assertion failed. Expected the default value for `{}` to be dbgenerated with `{:?}`, got `{:?}`",
                self.column.name(),
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
        let default = self.column.default().unwrap();

        assert!(matches!(default.kind(), DefaultKind::Value(PrismaValue::Enum(s)) if s == expected));

        self
    }

    pub fn assert_native_type(self, expected: &str, connector: &dyn Connector) -> Self {
        let found = connector
            .introspect_native_type(self.column.column_type().native_type.clone().unwrap())
            .to_string();
        assert!(
            found == expected,
            "Assertion failed. Expected the column native type for `{}` to be `{:?}`, found `{:?}`",
            self.column.name(),
            expected,
            found,
        );

        self
    }

    pub fn assert_type_family(self, expected: ColumnTypeFamily) -> Self {
        let found = self.column.column_type_family();

        let expected = match expected {
            ColumnTypeFamily::Enum(tbl_name) if self.tags.contains(Tags::LowerCasesTableNames) => {
                ColumnTypeFamily::Enum(tbl_name.to_lowercase())
            }
            _ => expected,
        };

        assert!(
            found == &expected,
            "Assertion failed. Expected the column type family for `{}` to be `{:?}`, found `{:?}`",
            self.column.name(),
            expected,
            found,
        );

        self
    }

    pub fn assert_type_is_bigint(self) -> Self {
        let found = self.column.column_type_family();

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::BigInt,
            "Assertion failed. Expected a BigInt column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_bytes(self) -> Self {
        let found = self.column.column_type_family();

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::Binary,
            "Assertion failed. Expected a bytes column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_decimal(self) -> Self {
        let found = self.column.column_type_family();

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::Decimal,
            "Assertion failed. Expected a decimal column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_enum(self) -> Self {
        let found = &self.column.column_type_family();

        assert!(
            matches!(found, sql_schema_describer::ColumnTypeFamily::Enum(_)),
            "Assertion failed. Expected an enum column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_string(self) -> Self {
        let found = self.column.column_type_family();

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::String,
            "Assertion failed. Expected a string column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_type_is_int(self) -> Self {
        let found = self.column.column_type_family();

        assert!(
            found == &sql_schema_describer::ColumnTypeFamily::Int,
            "Assertion failed. Expected an integer column, got {:?}.",
            found
        );

        self
    }

    pub fn assert_is_list(self) -> Self {
        assert!(
            self.column.arity().is_list(),
            "Assertion failed. Expected column `{}` to be a list, got {:?}",
            self.column.name(),
            self.column.arity(),
        );

        self
    }

    pub fn assert_is_nullable(self) -> Self {
        assert!(
            self.column.arity().is_nullable(),
            "Assertion failed. Expected column `{}` to be nullable, got {:?}",
            self.column.name(),
            self.column.arity(),
        );

        self
    }

    pub fn assert_is_required(self) -> Self {
        assert!(
            self.column.arity().is_required(),
            "Assertion failed. Expected column `{}` to be NOT NULL, got {:?}",
            self.column.name(),
            self.column.arity(),
        );

        self
    }
}

pub struct IndexColumnAssertion {
    sort_order: Option<SQLSortOrder>,
    length: Option<u32>,
    operator_class: Option<SQLOperatorClassKind>,
}

impl IndexColumnAssertion {
    #[track_caller]
    pub fn assert_sort_order(self, sort_order: SQLSortOrder) -> Self {
        assert_eq!(self.sort_order, Some(sort_order));

        self
    }

    #[track_caller]
    pub fn assert_length_prefix(self, length: u32) -> Self {
        assert_eq!(self.length, Some(length));

        self
    }

    #[track_caller]
    pub fn assert_no_length_prefix(self) -> Self {
        assert_eq!(self.length, None);
        self
    }

    #[track_caller]
    pub fn assert_ops(self, ops: SQLOperatorClassKind) -> Self {
        assert_eq!(self.operator_class, Some(ops));
        self
    }
}

pub struct PrimaryKeyAssertion<'a> {
    pk: IndexWalker<'a>,
    tags: BitFlags<Tags>,
}

impl<'a> PrimaryKeyAssertion<'a> {
    pub fn assert_columns(self, column_names: &[&str]) -> Self {
        assert_eq!(&self.pk.column_names().collect::<Vec<_>>(), column_names);

        self
    }

    pub fn assert_column<F>(self, column_name: &str, f: F) -> Self
    where
        F: FnOnce(IndexColumnAssertion) -> IndexColumnAssertion,
    {
        let col = self
            .pk
            .columns()
            .find(|c| c.name() == column_name)
            .unwrap_or_else(|| panic!("Could not find column {}", column_name));

        f(IndexColumnAssertion {
            length: col.length(),
            sort_order: col.sort_order(),
            operator_class: None,
        });

        self
    }

    #[track_caller]
    pub fn assert_has_autoincrement(self) -> Self {
        assert!(
            self.pk.columns().any(|column| column.as_column().is_autoincrement()
                || matches!(
                    column.as_column().default().map(|d| d.kind()),
                    Some(DefaultKind::UniqueRowid)
                )),
            "Assertion failed: expected a sequence on the primary key, found none."
        );

        self
    }

    pub fn assert_has_no_autoincrement(self) -> Self {
        assert!(
            !self.pk.columns().any(|column| column.as_column().is_autoincrement()),
            "Assertion failed: expected no sequence on the primary key, but found one."
        );

        self
    }

    pub fn assert_constraint_name(self, constraint_name: &str) -> Self {
        assert_eq!(self.pk.name(), constraint_name);
        self
    }

    #[track_caller]
    pub fn assert_non_clustered(self) -> Self {
        if self.tags.contains(Tags::Mssql) {
            let ext: &sql::mssql::MssqlSchemaExt = self.pk.schema.downcast_connector_data();
            assert!(!ext.index_is_clustered(self.pk.id))
        }

        self
    }

    #[track_caller]
    pub fn assert_clustered(self) -> Self {
        if self.tags.contains(Tags::Mssql) {
            let ext: &sql::mssql::MssqlSchemaExt = self.pk.schema.downcast_connector_data();
            assert!(ext.index_is_clustered(self.pk.id))
        }

        self
    }
}

pub struct ForeignKeyAssertion<'a> {
    fk: ForeignKeyWalker<'a>,
    tags: BitFlags<Tags>,
}

impl<'a> ForeignKeyAssertion<'a> {
    #[track_caller]
    pub fn assert_references(self, table: &str, columns: &[&str]) -> Self {
        assert!(
            self.is_same_table_name(self.fk.referenced_table().name(), table)
                && self.fk.referenced_columns().map(|c| c.name()).collect::<Vec<_>>() == columns,
            r#"Assertion failed. Expected reference to "{}" ({:?})."#,
            table,
            columns,
        );

        self
    }

    #[track_caller]
    pub fn assert_referential_action_on_delete(self, action: ForeignKeyAction) -> Self {
        assert!(
            self.fk.on_delete_action() == action,
            "Assertion failed: expected foreign key to {:?} on delete, but got {:?}.",
            action,
            self.fk.on_delete_action()
        );

        self
    }

    #[track_caller]
    pub fn assert_referential_action_on_update(self, action: ForeignKeyAction) -> Self {
        assert!(
            self.fk.on_update_action() == action,
            "Assertion failed: expected foreign key to {:?} on update, but got {:?}.",
            action,
            self.fk.on_update_action()
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

pub struct IndexAssertion<'a> {
    index: IndexWalker<'a>,
    tags: BitFlags<Tags>,
}

impl<'a> IndexAssertion<'a> {
    #[track_caller]
    pub fn assert_name(self, name: &str) -> Self {
        assert_eq!(self.index.name(), name);

        self
    }

    pub fn assert_is_fulltext(self) -> Self {
        assert_eq!(self.index.index_type(), IndexType::Fulltext);

        self
    }

    pub fn assert_is_normal(self) -> Self {
        assert_eq!(self.index.index_type(), IndexType::Normal);

        self
    }

    pub fn assert_is_unique(self) -> Self {
        assert_eq!(self.index.index_type(), IndexType::Unique);

        self
    }

    #[track_caller]
    pub fn assert_clustered(self) -> Self {
        if self.tags.contains(Tags::Mssql) {
            let ext: &sql::mssql::MssqlSchemaExt = self.index.schema.downcast_connector_data();
            assert!(ext.index_is_clustered(self.index.id))
        }

        self
    }

    #[track_caller]
    pub fn assert_non_clustered(self) -> Self {
        if self.tags.contains(Tags::Mssql) {
            let ext: &sql::mssql::MssqlSchemaExt = self.index.schema.downcast_connector_data();
            assert!(!ext.index_is_clustered(self.index.id))
        }

        self
    }

    pub fn assert_is_not_unique(self) -> Self {
        assert_eq!(self.index.index_type(), IndexType::Normal);

        self
    }

    pub fn assert_algorithm(self, algo: SqlIndexAlgorithm) -> Self {
        let postgres_ext: &PostgresSchemaExt = self.index.schema.downcast_connector_data();
        let algorithm = postgres_ext.index_algorithm(self.index.id);
        assert_eq!(algorithm, algo);

        self
    }

    pub fn assert_column<F>(self, column_name: &str, f: F) -> Self
    where
        F: FnOnce(IndexColumnAssertion) -> IndexColumnAssertion,
    {
        let col = self
            .index
            .columns()
            .find(|c| c.as_column().name() == column_name)
            .unwrap();

        let operator_class = if self.tags.contains(Tags::Postgres) {
            let ext: &PostgresSchemaExt = self.index.schema.downcast_connector_data();

            ext.get_opclass(col.id).map(|c| c.kind.clone())
        } else {
            None
        };

        f(IndexColumnAssertion {
            sort_order: col.sort_order(),
            length: col.length(),
            operator_class,
        });

        self
    }
}
