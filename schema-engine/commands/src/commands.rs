//! The commands exposed by the schema engine core are defined in this module.

mod apply_migrations;
mod create_migration;
mod dev_diagnostic;
mod diagnose_migration_history;
mod diff;
mod evaluate_data_loss;
mod introspect_sql;
mod mark_migration_applied;
mod mark_migration_rolled_back;
mod schema_push;

pub use diagnose_migration_history::{
    DiagnoseMigrationHistoryInput, DiagnoseMigrationHistoryOutput, DriftDiagnostic, HistoryDiagnostic,
};

pub use apply_migrations::apply_migrations;
pub use create_migration::create_migration;
pub use dev_diagnostic::dev_diagnostic;
pub use diagnose_migration_history::diagnose_migration_history;
pub use diff::diff;
pub use evaluate_data_loss::evaluate_data_loss;
pub use introspect_sql::introspect_sql;
pub use mark_migration_applied::mark_migration_applied;
pub use mark_migration_rolled_back::mark_migration_rolled_back;
pub use schema_push::schema_push;
