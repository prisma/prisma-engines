#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgresql")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use mysql::MysqlManager;
pub use postgres::PostgresManager;
pub use sqlite::SqliteManager;

use crate::connector::{SqliteParams, PostgresParams, MysqlParams};
use tokio_resource_pool::{Pool, Builder};
use std::convert::TryFrom;
use url::Url;

pub fn sqlite(path: &str) -> crate::Result<Pool<SqliteManager>> {
    let params = SqliteParams::try_from(path)?;
    let manager = SqliteManager::new(params.file_path);

    Ok(Builder::new().build(params.connection_limit as usize, manager))
}

pub fn postgres(url: Url) -> crate::Result<Pool<PostgresManager>> {
    let params = PostgresParams::try_from(url)?;
    let manager = PostgresManager::new(params.config, Some(params.schema), Some(params.ssl_params));

    Ok(Builder::new().build(params.connection_limit as usize, manager))
}

pub fn mysql(url: Url) -> crate::Result<Pool<MysqlManager>> {
    let params = MysqlParams::try_from(url)?;
    let manager = MysqlManager::new(params.config);

    Ok(Builder::new().build(params.connection_limit as usize, manager))
}
