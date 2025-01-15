//! The external facing programmatic API to the schema engine.

use crate::CoreResult;

/// The programmatic, generic, fantastic schema engine API.
///
/// TODO: should we even be constrained to such a generic API? Likely not.
/// E.g., version() only needs a single database connector, while diff() might need two.
/// Plus, subsequent calls to these API might depend on different database connectors that
/// must be passed in from the outer JavaScript layer.
#[async_trait::async_trait]
pub trait GenericApi: Send + Sync + 'static {
    /// Return the database version as a string.
    async fn version(&self, params: Option<GetDatabaseVersionInput>) -> CoreResult<String>;

    /// Make sure the connection to the database is established and valid.
    /// Connectors can choose to connect lazily, but this method should force
    /// them to connect.
    async fn ensure_connection_validity(
        &self,
        params: EnsureConnectionValidityParams,
    ) -> CoreResult<EnsureConnectionValidityResult>;
}
