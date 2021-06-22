mod config;
mod connector_tag;
mod datamodel_rendering;
mod error;
mod logging;
mod query_result;
mod runner;
mod templating;

pub use config::*;
pub use connector_tag::*;
pub use datamodel_rendering::*;
pub use error::*;
pub use logging::*;
pub use query_result::*;
pub use runner::*;
pub use templating::*;

use lazy_static::lazy_static;
use tokio::runtime::Builder;

pub type TestResult<T> = Result<T, TestError>;

lazy_static! {
    /// Test configuration, loaded once at runtime.
    pub static ref CONFIG: TestConfig = TestConfig::load().unwrap();
}

/// Teardown & setup of everything as defined in the passed datamodel.
pub async fn setup_project(datamodel: &str) -> TestResult<()> {
    Ok(migration_core::qe_setup::run(datamodel, Default::default()).await?)
}

/// Helper method to allow a sync shell function to run the async test blocks.
pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}
