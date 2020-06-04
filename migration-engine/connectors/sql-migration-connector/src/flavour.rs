//! SQL flavours implement behaviour specific to a given SQL implementation (PostgreSQL, SQLite...),
//! in order to avoid cluttering the connector with conditionals. This is a private implementation
//! detail of the SQL connector.

use crate::{database_info::DatabaseInfo, CheckDatabaseInfoResult, SqlResult, SystemDatabase};
use futures::{future::BoxFuture, FutureExt};
use once_cell::sync::Lazy;
use quaint::connector::Queryable;
use regex::RegexSet;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::{fs, path::PathBuf, sync::Arc};

pub(crate) fn from_database_info(database_info: &DatabaseInfo) -> Box<dyn SqlFlavour + Send + Sync + 'static> {
    use quaint::prelude::ConnectionInfo;

    match database_info.connection_info() {
        ConnectionInfo::Mysql(_) => Box::new(MysqlFlavour),
        ConnectionInfo::Postgres(_) => Box::new(PostgresFlavour),
        ConnectionInfo::Sqlite { file_path, .. } => Box::new(SqliteFlavour {
            file_path: file_path.clone(),
        }),
    }
}

#[async_trait::async_trait]
pub(crate) trait SqlFlavour {
    fn check_database_info(&self, _database_info: &DatabaseInfo) -> CheckDatabaseInfoResult {
        Ok(())
    }

    fn create_database<'a>(&'a self, _db_name: &'a str, _conn: &'a dyn Queryable) -> BoxFuture<'a, SqlResult<()>> {
        futures::future::ready(Ok(())).boxed()
    }

    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema>;

    async fn initialize(&self, conn: &dyn Queryable, database_info: &DatabaseInfo) -> SqlResult<()>;
}

struct MysqlFlavour;

#[async_trait::async_trait]
impl SqlFlavour for MysqlFlavour {
    fn check_database_info(&self, database_info: &DatabaseInfo) -> CheckDatabaseInfoResult {
        const MYSQL_SYSTEM_DATABASES: Lazy<regex::RegexSet> = Lazy::new(|| {
            RegexSet::new(&[
                "(?i)^mysql$",
                "(?i)^information_schema$",
                "(?i)^performance_schema$",
                "(?i)^sys$",
            ])
            .unwrap()
        });

        let db_name = database_info.connection_info().schema_name();

        if MYSQL_SYSTEM_DATABASES.is_match(db_name) {
            return Err(SystemDatabase(db_name.to_owned()));
        }

        Ok(())
    }

    fn create_database<'a>(&'a self, db_name: &'a str, conn: &'a dyn Queryable) -> BoxFuture<'a, SqlResult<()>> {
        async move {
            let query = format!("CREATE DATABASE `{}`", db_name);
            conn.query_raw(&query, &[]).await?;

            Ok(())
        }
        .boxed()
    }

    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::mysql::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn initialize(&self, conn: &dyn Queryable, database_info: &DatabaseInfo) -> SqlResult<()> {
        let schema_sql = format!(
            "CREATE SCHEMA IF NOT EXISTS `{}` DEFAULT CHARACTER SET latin1;",
            database_info.connection_info().schema_name()
        );

        conn.query_raw(&schema_sql, &[]).await?;

        Ok(())
    }
}

struct SqliteFlavour {
    file_path: String,
}

#[async_trait::async_trait]
impl SqlFlavour for SqliteFlavour {
    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::sqlite::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn initialize(&self, _conn: &dyn Queryable, _database_info: &DatabaseInfo) -> SqlResult<()> {
        let path_buf = PathBuf::from(&self.file_path);
        match path_buf.parent() {
            Some(parent_directory) => {
                fs::create_dir_all(parent_directory).expect("creating the database folders failed")
            }
            None => {}
        }

        Ok(())
    }
}

struct PostgresFlavour;

#[async_trait::async_trait]
impl SqlFlavour for PostgresFlavour {
    fn create_database<'a>(&'a self, db_name: &'a str, conn: &'a dyn Queryable) -> BoxFuture<'a, SqlResult<()>> {
        async move {
            let query = format!("CREATE DATABASE \"{}\"", db_name);
            conn.query_raw(&query, &[]).await?;

            Ok(())
        }
        .boxed()
    }

    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::postgres::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn initialize(&self, conn: &dyn Queryable, database_info: &DatabaseInfo) -> SqlResult<()> {
        let schema_sql = format!(
            "CREATE SCHEMA IF NOT EXISTS \"{}\";",
            &database_info.connection_info().schema_name()
        );

        conn.query_raw(&schema_sql, &[]).await?;

        Ok(())
    }
}
