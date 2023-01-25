#![deny(unsafe_code, rust_2018_idioms)]
#![allow(clippy::derive_partial_eq_without_eq)]

//! This crate contains constants and utilities that are useful for writing tests across the
//! engines.

pub mod mysql;
pub mod postgres;
/// Tokio test runtime utils.
pub mod runtime;

mod capabilities;
mod diff;
mod logging;
mod mssql;
mod sqlite;
mod tags;
mod test_api_args;

pub use capabilities::Capabilities;
pub use diff::panic_with_diff;
pub use enumflags2::BitFlags;
pub use mssql::reset_schema as reset_mssql_schema;
pub use sqlite::sqlite_test_url;
pub use tags::{tags_from_comma_separated_list, Tags};
pub use test_api_args::{DatasourceBlock, TestApiArgs};

type AnyError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[macro_export]
macro_rules! only {
    ($($tag:ident),*) => {
        ::test_setup::only!($($tag),* ; exclude: )
    };

    ($($tag:ident),* ; exclude: $($excludeTag:ident),*) => {
        {
            use ::test_setup::Tags;
            let (skip, db) = ::test_setup::only_impl(
                ::enumflags2::make_bitflags!(Tags::{$($tag)|*}),
                ::enumflags2::make_bitflags!(Tags::{$($excludeTag)|*})
            );
            if skip { return }
            db
        }
    };
}

pub struct TestDb(&'static test_api_args::DbUnderTest);

impl TestDb {
    pub fn url(&self) -> &'static str {
        &self.0.database_url
    }
}

#[doc(hidden)]
#[inline(never)]
pub fn only_impl(include_tagged: BitFlags<Tags>, exclude_tags: BitFlags<Tags>) -> (bool, TestDb) {
    let db = TestDb(test_api_args::db_under_test());
    if !include_tagged.intersects(db.0.tags) {
        println!("Test skipped");
        return (true, db);
    }

    if exclude_tags.intersects(db.0.tags) {
        println!("Test skipped");
        return (true, db);
    }

    (false, db)
}

#[inline(never)]
pub fn should_skip_test(
    include_tagged: BitFlags<Tags>,
    exclude_tags: BitFlags<Tags>,
    capabilities: BitFlags<Capabilities>,
) -> bool {
    let db = test_api_args::db_under_test();
    if !capabilities.is_empty() && !db.capabilities.contains(capabilities) {
        println!("Test skipped");
        return true;
    }

    if !include_tagged.is_empty() && !include_tagged.intersects(db.tags) {
        println!("Test skipped");
        return true;
    }

    if exclude_tags.intersects(db.tags) {
        println!("Test skipped");
        return true;
    }

    false
}
