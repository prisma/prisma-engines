mod mysql;
mod postgresql;
mod sqlite;
mod transaction;

use connector_interface::Connector;
use datamodel::Source;

pub use mysql::*;
pub use postgresql::*;
pub use sqlite::*;

pub trait FromSource {
    fn from_source(source: &dyn Source) -> crate::Result<Self>
    where
        Self: Connector + Sized;
}
