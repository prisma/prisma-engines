use crate::ConnectorResult;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Implementors of this trait are responsible for checking whether a migration
/// could lead to data loss, or if it would be potentially unexecutable.
///
/// The type parameter is the connector's
/// [DatabaseMigration](trait.MigrationConnector.html#associatedtype.DatabaseMigration)
/// type.
#[async_trait::async_trait]
pub trait DestructiveChangeChecker<T>: Send + Sync
where
    T: Send + Sync + 'static,
{
    /// Check destructive changes resulting of applying the provided migration.
    async fn check(&self, database_migration: &T) -> ConnectorResult<DestructiveChangeDiagnostics>;

    /// Check the database migration for destructive or unexecutable steps
    /// without performing any IO.
    fn pure_check(&self, database_migration: &T) -> DestructiveChangeDiagnostics;
}

/// The errors and warnings emitted by the [DestructiveChangeChecker](trait.DestructiveChangeChecker.html).
#[derive(Debug, Default)]
pub struct DestructiveChangeDiagnostics {
    /// The warnings.
    pub warnings: Vec<MigrationWarning>,
    /// Steps that are not executable.
    pub unexecutable_migrations: Vec<UnexecutableMigration>,
}

impl DestructiveChangeDiagnostics {
    /// Equivalent to Default::default()
    pub fn new() -> DestructiveChangeDiagnostics {
        Default::default()
    }

    /// Add a warning to the diagnostics.
    pub fn add_warning<T: Into<Option<MigrationWarning>>>(&mut self, warning: T) {
        if let Some(warning) = warning.into() {
            self.warnings.push(warning)
        }
    }

    /// Is there any warning to be rendered?
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// A warning emitted by [DestructiveChangeChecker](trait.DestructiveChangeChecker.html). Warnings will
/// prevent a migration from being applied, unless the `force` flag is passed.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct MigrationWarning {
    /// The user-facing warning description.
    pub description: String,
    /// The index of the step in the migration that this warning applies to.
    pub step_index: usize,
}

/// An unexecutable migration step detected by the DestructiveChangeChecker.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UnexecutableMigration {
    /// The user-facing problem description.
    pub description: String,
    /// The index of the step in the migration that this message applies to.
    pub step_index: usize,
}

/// An implementor of [DestructiveChangeChecker](trait.DestructiveChangeChecker.html) that performs no check.
#[derive(Default)]
pub struct EmptyDestructiveChangeChecker<T> {
    database_migration: PhantomData<T>,
}

#[async_trait::async_trait]
impl<T> DestructiveChangeChecker<T> for EmptyDestructiveChangeChecker<T>
where
    T: Send + Sync + 'static,
{
    async fn check(&self, _database_migration: &T) -> ConnectorResult<DestructiveChangeDiagnostics> {
        Ok(DestructiveChangeDiagnostics::new())
    }

    fn pure_check(&self, _database_migration: &T) -> DestructiveChangeDiagnostics {
        DestructiveChangeDiagnostics::new()
    }
}
