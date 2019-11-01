#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgresql")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use mysql::MysqlManager;
pub use postgres::PostgresManager;
pub use sqlite::SqliteManager;
pub use tokio_resource_pool::{CheckOut, Manage, Pool};

use crate::connector::{MysqlParams, PostgresParams, SqliteParams};
use std::convert::TryFrom;
use tokio_resource_pool::Builder;
use url::Url;

pub fn sqlite(path: &str, db_name: &str) -> crate::Result<Pool<SqliteManager>> {
    let params = SqliteParams::try_from(path)?;
    let manager = SqliteManager::new(params.file_path, db_name);

    #[cfg(not(feature = "tracing-log"))]
    {
        info!(
            "Starting an SQLite pool with {} connections.",
            params.connection_limit,
        );
    }
    #[cfg(feature = "tracing-log")]
    {
        tracing::info!(
            "Starting an SQLite pool with {} connections.",
            params.connection_limit,
        )
    }

    Ok(Builder::new().build(params.connection_limit as usize, manager))
}

pub fn postgres(url: Url) -> crate::Result<Pool<PostgresManager>> {
    let params = PostgresParams::try_from(url)?;
    let manager = PostgresManager::new(params.config, Some(params.schema), Some(params.ssl_params));

    #[cfg(not(feature = "tracing-log"))]
    {
        info!(
            "Starting a PostgreSQL pool with {} connections.",
            params.connection_limit,
        );
    }
    #[cfg(feature = "tracing-log")]
    {
        tracing::info!(
            "Starting a PostgreSQL pool with {} connections.",
            params.connection_limit,
        )
    }

    Ok(Builder::new().build(params.connection_limit as usize, manager))
}

pub fn mysql(url: Url) -> crate::Result<Pool<MysqlManager>> {
    let params = MysqlParams::try_from(url)?;
    let manager = MysqlManager::new(params.config);

    #[cfg(not(feature = "tracing-log"))]
    {
        info!(
            "Starting a MySQL pool with {} connections.",
            params.connection_limit,
        );
    }
    #[cfg(feature = "tracing-log")]
    {
        tracing::info!(
            "Starting a MySQL pool with {} connections.",
            params.connection_limit,
        )
    }

    Ok(Builder::new().build(params.connection_limit as usize, manager))
}
