//! Connection definitions for KingbaseES in Oracle-compatible mode.
//!
//! Kingbase Oracle mode uses the Kingbase PostgreSQL-wire driver, but keeps an
//! independent provider identity because its native types and SQL dialect are
//! not PostgreSQL or Kingbase MySQL semantics.

mod defaults;
mod error;
#[cfg(feature = "kingbase-oracle-native")]
pub(crate) mod native;
mod url;

pub use defaults::*;
pub use error::KingbaseOracleError;
#[cfg(feature = "kingbase-oracle-native")]
pub use native::KingbaseOracle;
pub use url::KingbaseOracleUrl;
