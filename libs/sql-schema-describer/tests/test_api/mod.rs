pub use quaint::{prelude::Queryable, single::Quaint};
pub use test_macros::test_connector;
pub use test_setup::{BitFlags, Capabilities, Tags};

use barrel::Migration;
use quaint::prelude::{ConnectionInfo, SqlFamily};
use sql_schema_describer::{
    postgres::Circumstances,
    walkers::{ColumnWalker, ForeignKeyWalker, IndexWalker, SqlSchemaExt, TableWalker},
    ColumnTypeFamily, DescriberError, ForeignKeyAction, SqlSchema, SqlSchemaDescriberBackend,
};
use test_setup::*;

pub struct TestApi {
    db_name: &'static str,
    database: Quaint,
    tags: BitFlags<Tags>,
}

impl TestApi {
    pub(crate) async fn new(args: TestApiArgs) -> Self {
        let tags = args.tags();
        let db_name = if tags.contains(Tags::Mysql) {
            test_setup::mysql_safe_identifier(args.test_function_name())
        } else {
            args.test_function_name()
        };

        let (conn, _connection_string) = if tags.contains(Tags::Mysql) {
            create_mysql_database(&db_name).await.unwrap()
        } else if tags.contains(Tags::Postgres) {
            create_postgres_database(&db_name).await.unwrap()
        } else if tags.contains(Tags::Mssql) {
            test_setup::init_mssql_database(args.database_url(), db_name)
                .await
                .unwrap()
        } else if tags.contains(Tags::Sqlite) {
            let url = sqlite_test_url(db_name);
            (Quaint::new(&url).await.unwrap(), url)
        } else {
            unreachable!()
        };

        TestApi {
            db_name,
            tags: args.tags(),
            database: conn,
        }
    }

    fn connection_info(&self) -> &ConnectionInfo {
        self.database.connection_info()
    }

    pub(crate) fn connector_tags(&self) -> BitFlags<Tags> {
        self.tags
    }

    pub(crate) async fn describe(&self) -> SqlSchema {
        self.describer().describe(self.schema_name()).await.unwrap()
    }

    pub(crate) async fn describe_error(&self) -> DescriberError {
        self.describer().describe(self.schema_name()).await.unwrap_err()
    }

    pub(crate) fn describer(&self) -> Box<dyn SqlSchemaDescriberBackend> {
        let db = self.database.clone();

        match self.sql_family() {
            SqlFamily::Postgres => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
                db,
                if self.tags.contains(Tags::Cockroach) {
                    Circumstances::Cockroach.into()
                } else {
                    Default::default()
                },
            )),
            SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(db)),
            SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(db)),
            SqlFamily::Mssql => Box::new(sql_schema_describer::mssql::SqlSchemaDescriber::new(db)),
        }
    }

    pub(crate) fn db_name(&self) -> &'static str {
        self.db_name
    }

    pub(crate) fn database(&self) -> &Quaint {
        &self.database
    }

    pub(crate) fn is_cockroach(&self) -> bool {
        self.tags.contains(Tags::Cockroach)
    }

    pub(crate) fn is_mariadb(&self) -> bool {
        self.tags.contains(Tags::Mariadb)
    }

    pub(crate) fn is_mssql(&self) -> bool {
        self.tags.contains(Tags::Mssql)
    }

    pub(crate) fn schema_name(&self) -> &str {
        match self.sql_family() {
            // It is not possible to connect to a specific schema in MSSQL. The
            // user has a dedicated schema from the admin, that's all.
            SqlFamily::Mssql => self.db_name(),
            _ => self.connection_info().schema_name(),
        }
    }

    pub(crate) fn sql_family(&self) -> SqlFamily {
        self.connection_info().sql_family()
    }

    pub(crate) fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            schema_name: self.schema_name().to_owned(),
            database: self.database.clone(),
            sql_variant: match self.sql_family() {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
                SqlFamily::Mssql => barrel::SqlVariant::Mssql,
            },
        }
    }
}

pub struct BarrelMigrationExecutor {
    pub(super) database: Quaint,
    pub(super) sql_variant: barrel::backend::SqlVariant,
    pub(super) schema_name: String,
}

impl BarrelMigrationExecutor {
    pub async fn execute<F>(&self, migration_fn: F)
    where
        F: FnOnce(&mut Migration),
    {
        self.execute_with_schema(migration_fn, &self.schema_name).await
    }

    pub async fn execute_with_schema<F>(&self, migration_fn: F, schema_name: &str)
    where
        F: FnOnce(&mut Migration),
    {
        let mut migration = Migration::new().schema(schema_name);
        migration_fn(&mut migration);

        let full_sql = migration.make_from(self.sql_variant);
        self.database.raw_cmd(&full_sql).await.unwrap();
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

    pub fn assert_foreign_keys_count(&self, expected_count: usize) -> &Self {
        assert_eq!(self.table.foreign_key_count(), expected_count);
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
                .find(|fk| fk.constrained_column_names() == cols)
                .unwrap(),
        };

        assertions(&fk);

        self
    }

    pub fn assert_index_on_columns(
        &self,
        columns: &[&str],
        assertions: impl for<'i> FnOnce(&'i IndexAssertion<'i>) -> &'i IndexAssertion<'i>,
    ) -> &Self {
        let index = self.table.indexes().find(|idx| idx.column_names() == columns).unwrap();

        assertions(&IndexAssertion { index });

        self
    }

    pub fn assert_indexes_count(&self, expected_count: usize) -> &Self {
        assert_eq!(self.table.indexes_count(), expected_count);
        self
    }

    pub fn assert_pk_on_columns(&self, columns: &[&str]) -> &Self {
        assert_eq!(self.table.primary_key().unwrap().columns, columns);
        self
    }
}

pub struct ColumnAssertion<'a> {
    column: ColumnWalker<'a>,
}

impl ColumnAssertion<'_> {
    pub fn assert_auto_increment(&self, expected: bool) -> &Self {
        assert_eq!(self.column.is_autoincrement(), expected);
        self
    }

    pub fn assert_column_type_family(&self, fam: ColumnTypeFamily) -> &Self {
        assert_eq!(self.column.column_type_family(), &fam);
        self
    }

    pub fn assert_full_data_type(&self, full_data_type: &str) -> &Self {
        assert_eq!(
            self.column.column().tpe.full_data_type,
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

    pub fn assert_no_default(&self) -> &Self {
        assert!(self.column.default().is_none());
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

    #[allow(unused)]
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
        assert!(self.index.index_type().is_unique());
        self
    }

    pub fn assert_is_not_unique(&self) -> &Self {
        assert!(!self.index.index_type().is_unique());
        self
    }
}

pub struct ForeignKeyAssertion<'a> {
    fk: ForeignKeyWalker<'a>,
}

impl<'a> ForeignKeyAssertion<'a> {
    pub fn assert_references(&self, table: &str, columns: &[&str]) -> &Self {
        assert_eq!(self.fk.referenced_table().name(), table);
        assert_eq!(self.fk.referenced_column_names(), columns);
        self
    }

    pub fn assert_on_delete(&self, expected: ForeignKeyAction) -> &Self {
        assert_eq!(self.fk.on_delete_action(), &expected);
        self
    }
}
