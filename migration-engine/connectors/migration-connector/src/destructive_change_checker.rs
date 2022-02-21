use crate::{BoxFuture, ConnectorResult, Migration};

/// Implementors of this trait are responsible for checking whether a migration
/// could lead to data loss, or if it would be potentially unexecutable.
///
/// The type parameter is the connector's
/// [DatabaseMigration](trait.MigrationConnector.html#associatedtype.DatabaseMigration)
/// type.
pub trait DestructiveChangeChecker: Send + Sync {
    /// Check destructive changes resulting of applying the provided migration.
    fn check<'a>(
        &'a mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, ConnectorResult<DestructiveChangeDiagnostics>>;

    /// Check the migration for destructive or unexecutable steps
    /// without performing any IO.
    fn pure_check(&self, migration: &Migration) -> DestructiveChangeDiagnostics;
}

/// The errors and warnings emitted by the
/// [DestructiveChangeChecker](trait.DestructiveChangeChecker.html).
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

    /// Is there any warning to be rendered?
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// A warning emitted by [DestructiveChangeChecker](trait.DestructiveChangeChecker.html). Warnings will
/// prevent a migration from being applied, unless the `force` flag is passed.
#[derive(Debug)]
pub struct MigrationWarning {
    /// The user-facing warning description.
    pub description: String,
    /// The index of the step in the migration that this warning applies to.
    pub step_index: usize,
}

/// An unexecutable migration step detected by the DestructiveChangeChecker.
#[derive(Debug)]
pub struct UnexecutableMigration {
    /// The user-facing problem description.
    pub description: String,
    /// The index of the step in the migration that this message applies to.
    pub step_index: usize,
}
