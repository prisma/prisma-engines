use barrel::Migration;
use quaint::{
    prelude::{Queryable, SqlFamily},
    single::Quaint,
};
use sql_schema_describer::*;
use std::sync::Arc;
use test_setup::*;

pub struct TestApi {
    /// More precise than SqlFamily.
    connector_name: &'static str,
    db_name: &'static str,
    connection_info: quaint::prelude::ConnectionInfo,
    sql_family: SqlFamily,
    database: Arc<dyn Queryable + Send + Sync + 'static>,
}

impl TestApi {
    pub(crate) async fn describe(&self) -> Result<SqlSchema, failure::Error> {
        let db = Arc::clone(&self.database);
        let describer: Box<dyn sql_schema_describer::SqlSchemaDescriberBackend> = match self.sql_family() {
            SqlFamily::Postgres => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(db)),
            SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(db)),
            SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(db)),
        };

        Ok(describer.describe(self.schema_name()).await?)
    }

    pub(crate) fn db_name(&self) -> &'static str {
        self.db_name
    }

    pub(crate) fn database(&self) -> &Arc<dyn Queryable + Send + Sync + 'static> {
        &self.database
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.connection_info.schema_name()
    }

    pub(crate) fn sql_family(&self) -> SqlFamily {
        self.sql_family
    }

    pub(crate) fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            schema_name: self.schema_name().to_owned(),
            database: Arc::clone(&self.database),
            sql_variant: match self.sql_family {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
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
        database: Arc::new(conn),
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
        database: Arc::new(conn),
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
        database: Arc::new(conn),
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
        database: Arc::new(conn),
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

pub async fn test_api_helper_for_postgres(url: String, db_name: &'static str, connector_name: &'static str) -> TestApi {
    let database = test_setup::create_postgres_database(&url.parse().unwrap())
        .await
        .unwrap();
    let connection_info = database.connection_info().to_owned();
    let drop_schema = dbg!(format!(
        "DROP SCHEMA IF EXISTS \"{}\" CASCADE;",
        connection_info.schema_name()
    ));
    database.query_raw(&drop_schema, &[]).await.ok();

    let create_schema = dbg!(format!(
        "CREATE SCHEMA IF NOT EXISTS \"{}\";",
        connection_info.schema_name()
    ));
    database.query_raw(&create_schema, &[]).await.ok();

    TestApi {
        connector_name,
        connection_info,
        db_name,
        database: Arc::new(database),
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
        database: Arc::new(database),
        sql_family: SqlFamily::Sqlite,
    }
}

pub struct BarrelMigrationExecutor {
    pub(super) database: Arc<dyn Queryable + Send + Sync>,
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

async fn run_full_sql(database: &Arc<dyn Queryable + Send + Sync>, full_sql: &str) {
    for sql in full_sql.split(";") {
        if sql != "" {
            database.query_raw(&sql, &[]).await.unwrap();
        }
    }
}
