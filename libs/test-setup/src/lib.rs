#![deny(unsafe_code, rust_2018_idioms)]

//! This crate contains constants and utilities that are useful for writing tests across the
//! engines.

/// Tokio test runtime utils.
pub mod runtime;

mod capabilities;
mod logging;
mod mssql;
mod mysql;
mod postgres;
mod sqlite;
mod tags;
mod test_api_args;

pub use capabilities::Capabilities;
pub use enumflags2::BitFlags;
pub use mssql::{init_mssql_database, reset_schema as reset_mssql_schema};
pub use sqlite::sqlite_test_url;
pub use tags::Tags;
pub use test_api_args::{DatasourceBlock, TestApiArgs};

type AnyError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[doc(hidden)]
pub fn should_skip_test(
    args: &TestApiArgs,
    include_tagged: BitFlags<Tags>,
    exclude_tags: BitFlags<Tags>,
    capabilities: BitFlags<Capabilities>,
) -> bool {
    if !capabilities.is_empty() && !args.capabilities().contains(capabilities) {
        println!("Test skipped");
        return true;
    }

    if !include_tagged.is_empty() && !include_tagged.intersects(args.tags()) {
        println!("Test skipped");
        return true;
    }

    if exclude_tags.intersects(args.tags()) {
        println!("Test skipped");
        return true;
    }

    false
}
