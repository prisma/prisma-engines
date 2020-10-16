#![deny(missing_docs)]

//! The commands exposed by the migration engine core are defined in this
//! module.

#[allow(missing_docs)]
mod apply_migration;
mod apply_migrations;
#[allow(missing_docs)]
mod calculate_database_steps;
#[allow(missing_docs)]
mod calculate_datamodel;
mod command;
mod create_migration;
mod debug_panic;
mod diagnose_migration_history;
mod evaluate_data_loss;
mod get_database_version;
#[allow(missing_docs)]
mod infer_migration_steps;
mod initialize;
#[allow(missing_docs)]
mod list_migrations;
#[allow(missing_docs)]
mod migration_progress;
mod plan_migration;
mod reset;
mod schema_push;
#[allow(missing_docs)]
mod unapply_migration;

pub use apply_migration::*;
pub use apply_migrations::{ApplyMigrationsCommand, ApplyMigrationsInput, ApplyMigrationsOutput};
pub use calculate_database_steps::*;
pub use calculate_datamodel::*;
pub use command::MigrationCommand;
pub use create_migration::{CreateMigrationCommand, CreateMigrationInput, CreateMigrationOutput};
pub use debug_panic::DebugPanicCommand;
pub use diagnose_migration_history::{
    DiagnoseMigrationHistoryCommand, DiagnoseMigrationHistoryInput, DiagnoseMigrationHistoryOutput, DriftDiagnostic,
    HistoryDiagnostic,
};
pub use evaluate_data_loss::*;
pub use get_database_version::*;
pub use infer_migration_steps::*;
pub use initialize::{InitializeCommand, InitializeInput, InitializeOutput};
pub use list_migrations::*;
pub use migration_progress::*;
pub use plan_migration::{PlanMigrationCommand, PlanMigrationInput, PlanMigrationOutput};
pub use reset::ResetCommand;
pub use schema_push::{SchemaPushCommand, SchemaPushInput, SchemaPushOutput};
pub use unapply_migration::*;

use migration_connector::{MigrationStep, MigrationWarning, PrettyDatabaseMigrationStep, UnexecutableMigration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct MigrationStepsResultOutput {
    pub datamodel: String,
    pub datamodel_steps: Vec<MigrationStep>,
    pub database_steps: Vec<PrettyDatabaseMigrationStep>,
    pub warnings: Vec<MigrationWarning>,
    pub errors: [(); 0],
    pub general_errors: [(); 0],
    pub unexecutable_migrations: Vec<UnexecutableMigration>,
}
