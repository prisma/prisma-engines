#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! This crate defines the API exposed by the connectors to the schema engine core. The entry point for this API is the [MigrationConnector](trait.MigrationConnector.html) trait.

mod checksum;
mod connector_host;
mod connector_params;
mod database_schema;
mod destructive_change_checker;
mod diff;
mod error;
mod introspection_context;
mod introspection_result;
mod migration;
mod migration_persistence;
mod namespaces;
mod schema_connector;

pub mod migrations_directory;
pub mod warnings;

pub use crate::namespaces::Namespaces;
pub use crate::schema_connector::SchemaConnector;
pub use connector_host::{ConnectorHost, EmptyHost};
pub use connector_params::ConnectorParams;
pub use database_schema::DatabaseSchema;
pub use destructive_change_checker::{
    DestructiveChangeChecker, DestructiveChangeDiagnostics, MigrationWarning, UnexecutableMigration,
};
pub use diff::DiffTarget;
pub use error::{ConnectorError, ConnectorResult};
pub use introspection_context::{CompositeTypeDepth, IntrospectionContext};
pub use introspection_result::{IntrospectionResult, ViewDefinition};
pub use migration::Migration;
pub use migration_persistence::{MigrationPersistence, MigrationRecord, PersistenceNotInitializedError, Timestamp};
pub use warnings::Warnings;

/// Alias for a pinned, boxed future, used by the traits.
pub type BoxFuture<'a, O> = std::pin::Pin<Box<dyn std::future::Future<Output = O> + Send + 'a>>;
