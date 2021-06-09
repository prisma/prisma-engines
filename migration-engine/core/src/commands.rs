//! The commands exposed by the migration engine core are defined in this
//! module.

mod apply_migrations;
mod command;
mod create_migration;
mod dev_diagnostic;
mod diagnose_migration_history;
mod evaluate_data_loss;
mod list_migration_directories;
mod mark_migration_applied;
mod mark_migration_rolled_back;
mod schema_push;

pub use apply_migrations::{ApplyMigrationsCommand, ApplyMigrationsInput, ApplyMigrationsOutput};
pub use command::MigrationCommand;
pub use create_migration::{CreateMigrationCommand, CreateMigrationInput, CreateMigrationOutput};
pub use dev_diagnostic::{DevAction, DevDiagnosticInput, DevDiagnosticOutput};
pub use diagnose_migration_history::{
    DiagnoseMigrationHistoryInput, DiagnoseMigrationHistoryOutput, DriftDiagnostic, HistoryDiagnostic,
};
pub use evaluate_data_loss::*;
pub use list_migration_directories::*;
pub use mark_migration_applied::{MarkMigrationAppliedCommand, MarkMigrationAppliedInput, MarkMigrationAppliedOutput};
pub use mark_migration_rolled_back::{MarkMigrationRolledBackInput, MarkMigrationRolledBackOutput};
pub use schema_push::{SchemaPushCommand, SchemaPushInput, SchemaPushOutput};

pub(crate) use dev_diagnostic::dev_diagnostic;
pub(crate) use diagnose_migration_history::diagnose_migration_history;
pub(crate) use mark_migration_rolled_back::mark_migration_rolled_back;
