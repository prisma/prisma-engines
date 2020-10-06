#![allow(dead_code)]

use barrel::Migration;
use quaint::{
    prelude::{Queryable, SqlFamily},
    single::Quaint,
};
use sql_schema_describer::*;
use test_setup::*;

pub type TestResult = anyhow::Result<()>;

pub struct TestApi {
    /// More precise than SqlFamily.
    connector_name: &'static str,
    db_name: &'static str,
    connection_info: quaint::prelude::ConnectionInfo,
    sql_family: SqlFamily,
    database: Quaint,
}

impl TestApi {
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
        match self.sql_family {
            // It is not possible to connect to a specific schema in MSSQL. The
            // user has a dedicated schema from the admin, that's all.
            SqlFamily::Mssql => self.db_name(),
            _ => self.connection_info.schema_name(),
        }
    }

    pub(crate) fn sql_family(&self) -> SqlFamily {
        self.sql_family
    }

    pub(crate) fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            schema_name: self.schema_name().to_owned(),
            database: self.database.clone(),
            sql_variant: match self.sql_family {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
                SqlFamily::Mssql => barrel::SqlVariant::Mssql,
            },
        }
    }

    pub(crate) fn connector_name(&self) -> &'static str {
        self.connector_name
    }
}

pub async fn mysql_test_api(db_name: &'static str) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(db_name);
    let url = mysql_url(db_name.as_ref());
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    TestApi {
        connector_name: "mysql5.7",
        connection_info: conn.connection_info().to_owned(),
        db_name,
        database: conn,
        sql_family: SqlFamily::Mysql,
    }
}

pub async fn mysql_8_test_api(db_name: &'static str) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(db_name);
    let url = mysql_8_url(db_name.as_ref());
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    TestApi {
        connector_name: "mysql8",
        connection_info: conn.connection_info().to_owned(),
        db_name,
        database: conn,
        sql_family: SqlFamily::Mysql,
    }
}

pub async fn mysql_5_6_test_api(db_name: &'static str) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(db_name);
    let url = mysql_5_6_url(db_name.as_ref());
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    TestApi {
        connector_name: "mysql_5_6",
        connection_info: conn.connection_info().to_owned(),
        db_name,
        database: conn,
        sql_family: SqlFamily::Mysql,
    }
}

pub async fn mysql_mariadb_test_api(db_name: &'static str) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(db_name);
    let url = mariadb_url(db_name.as_ref());
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    TestApi {
        connector_name: "mysql_mariadb",
        db_name,
        connection_info: conn.connection_info().to_owned(),
        database: conn,
        sql_family: SqlFamily::Mysql,
    }
}

pub async fn postgres_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_10_url(db_name), db_name, "postgres10").await
}

pub async fn postgres9_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_9_url(db_name), db_name, "postgres9").await
}

pub async fn postgres11_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_11_url(db_name), db_name, "postgres11").await
}

pub async fn postgres12_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_12_url(db_name), db_name, "postgres12").await
}

pub async fn postgres13_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_13_url(db_name), db_name, "postgres13").await
}

pub async fn test_api_helper_for_postgres(url: String, db_name: &'static str, connector_name: &'static str) -> TestApi {
    let database = test_setup::create_postgres_database(&url.parse().unwrap())
        .await
        .unwrap();
    let connection_info = database.connection_info().to_owned();
    let drop_schema = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", connection_info.schema_name());
    database.query_raw(&drop_schema, &[]).await.ok();

    let create_schema = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", connection_info.schema_name());

    database.query_raw(&create_schema, &[]).await.ok();

    TestApi {
        connector_name,
        connection_info,
        db_name,
        database,
        sql_family: SqlFamily::Postgres,
    }
}

pub async fn sqlite_test_api(db_name: &'static str) -> TestApi {
    let database_file_path = sqlite_test_file(db_name);
    std::fs::remove_file(database_file_path.clone()).ok(); // ignore potential errors
    let connection_string = sqlite_test_url(db_name);
    let database = Quaint::new(&connection_string).await.unwrap();

    TestApi {
        connector_name: "sqlite3",
        db_name,
        connection_info: database.connection_info().to_owned(),
        database,
        sql_family: SqlFamily::Sqlite,
    }
}

pub async fn mssql_2017_test_api(schema: &'static str) -> TestApi {
    mssql_test_api(mssql_2017_url("master"), schema, "mssql2017").await
}

pub async fn mssql_2019_test_api(schema: &'static str) -> TestApi {
    mssql_test_api(mssql_2019_url("master"), schema, "mssql2017").await
}

pub async fn mssql_test_api(connection_string: String, schema: &'static str, connector_name: &'static str) -> TestApi {
    use test_setup::connectors::mssql;

    let database = Quaint::new(&connection_string).await.unwrap();
    let connection_info = database.connection_info().to_owned();

    mssql::reset_schema(&database, schema).await.unwrap();

    TestApi {
        connector_name,
        db_name: schema,
        connection_info,
        database,
        sql_family: SqlFamily::Mssql,
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
        F: FnOnce(&mut Migration) -> (),
    {
        self.execute_with_schema(migration_fn, &self.schema_name).await
    }

    pub async fn execute_with_schema<F>(&self, migration_fn: F, schema_name: &str)
    where
        F: FnOnce(&mut Migration) -> (),
    {
        let mut migration = Migration::new().schema(schema_name);
        migration_fn(&mut migration);

        let full_sql = migration.make_from(self.sql_variant);
        run_full_sql(&self.database, &full_sql).await;
    }
}

async fn run_full_sql(database: &Quaint, full_sql: &str) {
    for sql in full_sql.split(";") {
        if sql != "" {
            database.query_raw(&sql, &[]).await.unwrap();
        }
    }
}
