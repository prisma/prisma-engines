pub use expect_test::expect;
pub use indoc::{formatdoc, indoc};
pub use quaint::{prelude::Queryable, single::Quaint};
pub use test_macros::test_connector;
pub use test_setup::{runtime::run_with_thread_local_runtime as tok, BitFlags, Capabilities, Tags};

use barrel::Migration;
use quaint::prelude::SqlFamily;
use sql_schema_describer::{
    postgres::Circumstances,
    walkers::{ColumnWalker, ForeignKeyWalker, IndexWalker, SqlSchemaExt, TableWalker},
    ColumnTypeFamily, DescriberError, ForeignKeyAction, SqlSchema, SqlSchemaDescriberBackend,
};
use std::future::Future;
use test_setup::*;

pub struct TestApi {
    db_name: &'static str,
    database: Quaint,
    tags: BitFlags<Tags>,
}

impl TestApi {
    pub(crate) fn new(args: TestApiArgs) -> Self {
        let tags = args.tags();
        let (db_name, conn) = if tags.contains(Tags::Mysql) {
            let (db_name, cs) = tok(args.create_mysql_database());
            (db_name, tok(Quaint::new(&cs)).unwrap())
        } else if tags.contains(Tags::Postgres) {
            let (db_name, q, _) = tok(args.create_postgres_database());
            if tags.contains(Tags::CockroachDb) {
                tok(q.raw_cmd(
                    r#"
                    SET default_int_size = 4;
                    SET serial_normalization = 'sql_sequence';
                    "#,
                ))
                .unwrap();
            }
            (db_name, q)
        } else if tags.contains(Tags::Mssql) {
            let (q, _cs) = tok(args.create_mssql_database());
            (args.test_function_name(), q)
        } else if tags.contains(Tags::Sqlite) {
            (args.test_function_name(), Quaint::new_in_memory().unwrap())
        } else {
            unreachable!()
        };

        TestApi {
            db_name,
            tags: args.tags(),
            database: conn,
        }
    }

    pub(crate) fn expect_schema(&self, expected_schema: expect_test::Expect) {
        let schema = self.describe();
        expected_schema.assert_debug_eq(&schema);
    }

    pub(crate) fn block_on<O>(&self, f: impl Future<Output = O>) -> O {
        tok(f)
    }

    pub(crate) fn connector_tags(&self) -> BitFlags<Tags> {
        self.tags
    }

    pub(crate) fn describe(&self) -> SqlSchema {
        self.describe_with_schema(self.schema_name())
    }

    pub(crate) fn describe_with_schema(&self, schema: &str) -> SqlSchema {
        tok(self.describer(&self.database).describe(schema)).unwrap()
    }

    pub(crate) fn describe_error(&self) -> DescriberError {
        tok(self.describer(&self.database).describe(self.schema_name())).unwrap_err()
    }

    fn describer<'a>(&self, connection: &'a dyn Queryable) -> Box<dyn SqlSchemaDescriberBackend + 'a> {
        match self.sql_family() {
            SqlFamily::Postgres => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
                connection,
                if self.tags.contains(Tags::CockroachDb) {
                    Circumstances::Cockroach.into()
                } else {
                    Default::default()
                },
            )),
            SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(connection)),
            SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(connection)),
            SqlFamily::Mssql => Box::new(sql_schema_describer::mssql::SqlSchemaDescriber::new(connection)),
        }
    }

    pub(crate) fn db_name(&self) -> &'static str {
        self.db_name
    }

    pub(crate) fn database(&self) -> &Quaint {
        &self.database
    }

    pub(crate) fn execute_barrel(&self, migration_fn: impl FnOnce(&mut Migration)) {
        let mut migration = Migration::new().schema(self.schema_name());
        migration_fn(&mut migration);

        let full_sql = migration.make_from(match self.sql_family() {
            SqlFamily::Mysql => barrel::SqlVariant::Mysql,
            SqlFamily::Postgres => barrel::SqlVariant::Pg,
            SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
            SqlFamily::Mssql => barrel::SqlVariant::Mssql,
        });
        tok(self.database.raw_cmd(&full_sql)).unwrap();
    }

    pub(crate) fn schema_name(&self) -> &str {
        match self.sql_family() {
            // It is not possible to connect to a specific schema in MSSQL. The
            // user has a dedicated schema from the admin, that's all.
            SqlFamily::Mssql => self.db_name(),
            _ => self.database.connection_info().schema_name(),
        }
    }

    #[track_caller]
    pub(crate) fn raw_cmd(&self, sql: &str) {
        tok(self.database.raw_cmd(sql)).unwrap()
    }

    pub(crate) fn sql_family(&self) -> SqlFamily {
        self.database.connection_info().sql_family()
    }
}

pub trait SqlSchemaAssertionsExt {
    fn assert_table(
        &self,
        table_name: &str,
        assertions: impl for<'a> FnOnce(&'a TableAssertion<'a>) -> &'a TableAssertion<'a>,
    ) -> &Self;
}

impl SqlSchemaAssertionsExt for SqlSchema {
    fn assert_table(
        &self,
        table_name: &str,
        assertions: impl for<'a> FnOnce(&'a TableAssertion<'a>) -> &'a TableAssertion<'a>,
    ) -> &Self {
        let mut table = TableAssertion {
            table: self.table_walker(table_name).unwrap(),
        };

        assertions(&mut table);

        self
    }
}

pub struct TableAssertion<'a> {
    table: TableWalker<'a>,
}

impl TableAssertion<'_> {
    pub fn assert_column(
        &self,
        column_name: &str,
        assertions: impl for<'c> FnOnce(&'c ColumnAssertion<'c>) -> &'c ColumnAssertion<'c>,
    ) -> &Self {
        let mut column = ColumnAssertion {
            column: self
                .table
                .column(column_name)
                .ok_or_else(|| format!("Could not find the {} column", column_name))
                .unwrap(),
        };

        assertions(&mut column);

        self
    }

    pub fn assert_foreign_key_on_columns(
        &self,
        cols: &[&str],
        assertions: impl for<'fk> FnOnce(&'fk ForeignKeyAssertion<'fk>) -> &'fk ForeignKeyAssertion<'fk>,
    ) -> &Self {
        let fk = ForeignKeyAssertion {
            fk: self
                .table
                .foreign_keys()
                .find(|fk| {
                    let constrained_columns = fk.constrained_columns();
                    constrained_columns.len() == cols.len()
                        && constrained_columns.zip(cols).all(|(a, b)| a.name() == *b)
                })
                .unwrap(),
        };

        assertions(&fk);

        self
    }

    #[track_caller]
    pub fn assert_index_on_columns(
        &self,
        columns: &[&str],
        assertions: impl for<'i> FnOnce(&'i IndexAssertion<'i>) -> &'i IndexAssertion<'i>,
    ) -> &Self {
        let index = self
            .table
            .indexes()
            .find(|i| {
                let lengths_match = i.columns().len() == columns.len();
                let columns_match = i.columns().zip(columns.iter()).all(|(a, b)| a.as_column().name() == *b);

                lengths_match && columns_match
            })
            .unwrap();

        assertions(&IndexAssertion { index });

        self
    }

    pub fn assert_pk_on_columns(&self, columns: &[&str]) -> &Self {
        let pk_columns = self
            .table
            .primary_key()
            .unwrap()
            .columns()
            .map(|c| c.name())
            .collect::<Vec<_>>();

        assert_eq!(pk_columns, columns);

        self
    }
}

pub struct ColumnAssertion<'a> {
    column: ColumnWalker<'a>,
}

impl ColumnAssertion<'_> {
    pub fn assert_column_type_family(&self, fam: ColumnTypeFamily) -> &Self {
        assert_eq!(self.column.column_type_family(), &fam);
        self
    }

    pub fn assert_full_data_type(&self, full_data_type: &str) -> &Self {
        assert_eq!(
            self.column.column_type().full_data_type,
            full_data_type,
            "assert_full_data_type() for {}",
            self.column.name()
        );
        self
    }

    pub fn assert_is_list(&self) -> &Self {
        assert!(self.column.arity().is_list());
        self
    }

    pub fn assert_not_null(&self) -> &Self {
        assert!(self.column.arity().is_required());
        self
    }

    pub fn assert_nullable(&self) -> &Self {
        assert!(self.column.arity().is_nullable());
        self
    }

    pub fn assert_type_is_int_or_bigint(&self) -> &Self {
        let fam = self.column.column_type_family();
        assert!(fam.is_int() || fam.is_bigint(), "Expected int or bigint, got {:?}", fam);
        self
    }

    pub fn assert_type_is_int(&self) -> &Self {
        assert!(self.column.column_type_family().is_int());
        self
    }

    pub fn assert_type_is_string(&self) -> &Self {
        assert!(self.column.column_type_family().is_string());
        self
    }
}

pub struct IndexAssertion<'a> {
    index: IndexWalker<'a>,
}

impl IndexAssertion<'_> {
    pub fn assert_name(&self, name: &str) -> &Self {
        assert_eq!(self.index.name(), name);
        self
    }

    pub fn assert_is_unique(&self) -> &Self {
        assert!(self.index.is_unique());
        self
    }

    pub fn assert_is_not_unique(&self) -> &Self {
        assert!(!self.index.is_unique());
        self
    }
}

pub struct ForeignKeyAssertion<'a> {
    fk: ForeignKeyWalker<'a>,
}

impl<'a> ForeignKeyAssertion<'a> {
    pub fn assert_references(&self, table: &str, columns: &[&str]) -> &Self {
        assert_eq!(self.fk.referenced_table().name(), table);
        let referenced_columns = self.fk.referenced_columns();
        assert_eq!(referenced_columns.len(), columns.len());
        for (a, b) in referenced_columns.zip(columns.iter()) {
            assert_eq!(a.name(), *b);
        }
        self
    }

    pub fn assert_on_delete(&self, expected: ForeignKeyAction) -> &Self {
        assert_eq!(self.fk.on_delete_action(), expected);
        self
    }
}
