#![allow(dead_code)]

use barrel::Migration;
use enumflags2::BitFlags;
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_schema_describer::*;
use test_setup::connectors::Tags;
use test_setup::*;

pub type TestResult = anyhow::Result<()>;

pub struct TestApi {
    db_name: &'static str,
    database: Quaint,
    tags: BitFlags<Tags>,
}

impl TestApi {
    pub(crate) async fn new(args: TestApiArgs) -> Self {
        let tags = args.connector_tags;
        let db_name = if args.connector_tags.contains(Tags::Mysql) {
            test_setup::mysql_safe_identifier(args.test_function_name)
        } else {
            args.test_function_name
        };

        let url = (args.url_fn)(db_name);
        let url = if tags.contains(Tags::Mssql) {
            format!("{};schema={}", url, db_name)
        } else {
            url
        };

        let conn = if tags.contains(Tags::Mysql) {
            create_mysql_database(&url.parse().unwrap()).await.unwrap()
        } else if tags.contains(Tags::Postgres) {
            create_postgres_database(&url.parse().unwrap()).await.unwrap()
        } else if tags.contains(Tags::Mssql) {
            let conn = create_mssql_database(&url).await.unwrap();

            test_setup::connectors::mssql::reset_schema(&conn, db_name)
                .await
                .unwrap();

            conn
        } else if tags.contains(Tags::Sqlite) {
            Quaint::new(&url).await.unwrap()
        } else {
            unreachable!()
        };

        TestApi {
            db_name,
            tags: args.connector_tags,
            database: conn,
        }
    }

    fn connection_info(&self) -> &ConnectionInfo {
        self.database.connection_info()
    }

    pub(crate) fn connector_tags(&self) -> BitFlags<Tags> {
        self.tags
    }

    pub(crate) async fn describe(&self) -> Result<SqlSchema, anyhow::Error> {
        let db = self.database.clone();
        let describer: Box<dyn sql_schema_describer::SqlSchemaDescriberBackend> = match self.sql_family() {
            SqlFamily::Postgres => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(db)),
            SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(db)),
            SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(db)),
            SqlFamily::Mssql => Box::new(sql_schema_describer::mssql::SqlSchemaDescriber::new(db)),
        };

        Ok(describer.describe(self.schema_name()).await?)
    }

    pub(crate) fn db_name(&self) -> &'static str {
        self.db_name
    }

    pub(crate) fn database(&self) -> &Quaint {
        &self.database
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
