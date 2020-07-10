use crate::ConnectorResult;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Implementors of this trait are responsible for checking whether a migration could lead to data loss.
///
/// The type parameter is the connector's [DatabaseMigration](trait.MigrationConnector.html#associatedtype.DatabaseMigration)
/// type.
#[async_trait::async_trait]
pub trait DestructiveChangesChecker<T>: Send + Sync
where
    T: Send + Sync + 'static,
{
    /// Check destructive changes resulting of applying the provided migration.
    async fn check(&self, database_migration: &T) -> ConnectorResult<DestructiveChangeDiagnostics>;

    /// Check destructive changes resulting of reverting the provided migration.
    async fn check_unapply(&self, database_migration: &T) -> ConnectorResult<DestructiveChangeDiagnostics>;
}

/// The errors and warnings emitted by the [DestructiveChangesChecker](trait.DestructiveChangesChecker.html).
#[derive(Debug)]
pub struct DestructiveChangeDiagnostics {
    pub errors: Vec<MigrationError>,
    pub warnings: Vec<MigrationWarning>,
    pub unexecutable_migrations: Vec<UnexecutableMigration>,
}

impl DestructiveChangeDiagnostics {
    pub fn new() -> DestructiveChangeDiagnostics {
        DestructiveChangeDiagnostics {
            errors: Vec::new(),
            warnings: Vec::new(),
            unexecutable_migrations: Vec::new(),
        }
    }

    pub fn add_warning<T: Into<Option<MigrationWarning>>>(&mut self, warning: T) {
        if let Some(warning) = warning.into() {
            self.warnings.push(warning)
        }
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn warn_about_unexecutable_migrations(&mut self) {
        for unexecutable in &self.unexecutable_migrations {
            self.warnings.push(MigrationWarning {
                description: unexecutable.description.clone(),
            });
        }
    }
}

/// A warning emitted by [DestructiveChangesChecker](trait.DestructiveChangesChecker.html). Warnings will
/// prevent a migration from being applied, unless the `force` flag is passed.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct MigrationWarning {
    pub description: String,
}

/// An error emitted by the [DestructiveChangesChecker](trait.DestructiveChangesChecker.html). Errors will
/// always prevent a migration from being applied.
#[derive(Debug, Serialize, PartialEq, Deserialize)]
pub struct MigrationError {
    pub tpe: String,
    pub description: String,
    pub field: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UnexecutableMigration {
    pub description: String,
}

/// An implementor of [DestructiveChangesChecker](trait.DestructiveChangesChecker.html) that performs no check.
pub struct EmptyDestructiveChangesChecker<T> {
    database_migration: PhantomData<T>,
}

impl<T> EmptyDestructiveChangesChecker<T> {
    pub fn new() -> EmptyDestructiveChangesChecker<T> {
        EmptyDestructiveChangesChecker {
            database_migration: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T> DestructiveChangesChecker<T> for EmptyDestructiveChangesChecker<T>
where
    T: Send + Sync + 'static,
{
    async fn check(&self, _database_migration: &T) -> ConnectorResult<DestructiveChangeDiagnostics> {
        Ok(DestructiveChangeDiagnostics::new())
    }

    async fn check_unapply(&self, _database_migration: &T) -> ConnectorResult<DestructiveChangeDiagnostics> {
        Ok(DestructiveChangeDiagnostics::new())
    }
}
