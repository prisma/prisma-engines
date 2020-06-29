mod builtin_datasource_providers;
mod datasource;
mod datasource_provider;
mod loader;
mod serializer;

//pub mod builtin;

#[cfg(feature = "mssql")]
pub use builtin::MSSQL_SOURCE_NAME;
// TODO: i think these constants should move to a more central place.
//pub use builtin::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME};
pub use builtin_datasource_providers::*;
pub use datasource::*;
pub use datasource_provider::*;
pub use loader::*;
pub use serializer::*;
