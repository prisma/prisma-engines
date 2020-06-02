//! SQL flavours implement behaviour specific to a given SQL implementation (PostgreSQL, SQLite...),
//! in order to avoid cluttering the connector with conditionals. This is a private implementation
//! detail of the SQL connector.

use crate::{database_info::DatabaseInfo, SqlResult};
use futures::{future::BoxFuture, FutureExt};
use quaint::connector::Queryable;
use std::{fs, path::PathBuf};

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

pub trait SqlFlavour {
    fn create_database<'a>(&'a self, _db_name: &'a str, _conn: &'a dyn Queryable) -> BoxFuture<'a, SqlResult<()>> {
        async { Ok(()) }.boxed()
    }

    fn initialize<'a>(
        &'a self,
        conn: &'a dyn Queryable,
        database_info: &'a DatabaseInfo,
    ) -> BoxFuture<'a, SqlResult<()>>;
}

struct MysqlFlavour;

impl SqlFlavour for MysqlFlavour {
    fn create_database<'a>(&'a self, db_name: &'a str, conn: &'a dyn Queryable) -> BoxFuture<'a, SqlResult<()>> {
        async move {
            let query = format!("CREATE DATABASE `{}`", db_name);
            conn.query_raw(&query, &[]).await?;

            Ok(())
        }
        .boxed()
    }

    fn initialize<'a>(
        &'a self,
        conn: &'a dyn Queryable,
        database_info: &'a DatabaseInfo,
    ) -> BoxFuture<'a, SqlResult<()>> {
        async move {
            let schema_sql = format!(
                "CREATE SCHEMA IF NOT EXISTS `{}` DEFAULT CHARACTER SET latin1;",
                database_info.connection_info().schema_name()
            );

            conn.query_raw(&schema_sql, &[]).await?;

            Ok(())
        }
        .boxed()
    }
}

struct SqliteFlavour {
    file_path: String,
}

impl SqlFlavour for SqliteFlavour {
    fn initialize<'a>(
        &'a self,
        _conn: &'a dyn Queryable,
        _database_info: &'a DatabaseInfo,
    ) -> BoxFuture<'a, SqlResult<()>> {
        let path_buf = PathBuf::from(&self.file_path);
        match path_buf.parent() {
            Some(parent_directory) => {
                fs::create_dir_all(parent_directory).expect("creating the database folders failed")
            }
            None => {}
        }

        futures::future::ready(Ok(())).boxed()
    }
}

struct PostgresFlavour;

impl SqlFlavour for PostgresFlavour {
    fn create_database<'a>(&'a self, db_name: &'a str, conn: &'a dyn Queryable) -> BoxFuture<'a, SqlResult<()>> {
        async move {
            let query = format!("CREATE DATABASE \"{}\"", db_name);
            conn.query_raw(&query, &[]).await?;

            Ok(())
        }
        .boxed()
    }
    fn initialize<'a>(
        &'a self,
        conn: &'a dyn Queryable,
        database_info: &'a DatabaseInfo,
    ) -> BoxFuture<'a, SqlResult<()>> {
        async move {
            let schema_sql = format!(
                "CREATE SCHEMA IF NOT EXISTS \"{}\";",
                &database_info.connection_info().schema_name()
            );

            conn.query_raw(&schema_sql, &[]).await?;

            Ok(())
        }
        .boxed()
    }
}
