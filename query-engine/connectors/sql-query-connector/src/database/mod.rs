mod connection;
mod mysql;
mod postgresql;
mod sqlite;
mod transaction;

pub(crate) mod operations;

use connector_interface::Connector;
use datamodel::Source;
use async_trait::async_trait;

pub use mysql::*;
pub use postgresql::*;
pub use sqlite::*;

#[async_trait]
pub trait FromSource {
    async fn from_source(source: &dyn Source) -> crate::Result<Self>
    where
        Self: Connector + Sized;
}
