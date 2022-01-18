//! The commands exposed by the migration engine core are defined in this
//! module.

mod apply_migrations;
mod create_migration;
mod dev_diagnostic;
mod diagnose_migration_history;
mod evaluate_data_loss;
mod mark_migration_applied;
mod mark_migration_rolled_back;
mod schema_push;

pub use create_migration::{CreateMigrationInput, CreateMigrationOutput};
pub use dev_diagnostic::{DevAction, DevDiagnosticInput, DevDiagnosticOutput};
pub use diagnose_migration_history::{
    DiagnoseMigrationHistoryInput, DiagnoseMigrationHistoryOutput, DriftDiagnostic, HistoryDiagnostic,
};
pub use evaluate_data_loss::*;
pub use schema_push::{SchemaPushInput, SchemaPushOutput};

pub(crate) use apply_migrations::apply_migrations;
pub(crate) use create_migration::create_migration;
pub(crate) use dev_diagnostic::dev_diagnostic;
pub(crate) use diagnose_migration_history::diagnose_migration_history;
pub(crate) use mark_migration_applied::mark_migration_applied;
pub(crate) use mark_migration_rolled_back::mark_migration_rolled_back;
pub(crate) use schema_push::schema_push;
