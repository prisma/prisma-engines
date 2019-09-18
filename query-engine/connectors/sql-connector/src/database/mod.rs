mod transaction;
mod mysql;
mod postgresql;
mod sqlite;

use datamodel::Source;
use connector_interface::Connector;

pub use mysql::*;
pub use postgresql::*;
pub use sqlite::*;

pub trait FromSource {
    fn from_source(source: &dyn Source) -> crate::Result<Self>
    where
        Self: Connector + Sized;
}
