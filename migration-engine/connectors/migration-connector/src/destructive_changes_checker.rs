use crate::ConnectorResult;
use serde::Serialize;
use std::marker::PhantomData;

/// Implementors of this trait are responsible for checking whether a migration could lead to data loss.
///
/// The type parameter is the connector's [DatabaseMigration](trait.MigrationConnector.html#associatedtype.DatabaseMigration)
/// type.
pub trait DestructiveChangesChecker<T>: Send + Sync + 'static
where
    T: Send + Sync + 'static,
{
    fn check(&self, database_migration: &T) -> ConnectorResult<DestructiveChangeDiagnostics>;
}

/// The errors and warnings emitted by the [DestructiveChangesChecker](trait.DestructiveChangesChecker.html).
#[derive(Debug)]
pub struct DestructiveChangeDiagnostics {
    pub errors: Vec<MigrationError>,
    pub warnings: Vec<MigrationWarning>,
}

impl DestructiveChangeDiagnostics {
    pub fn new() -> DestructiveChangeDiagnostics {
        DestructiveChangeDiagnostics {
            errors: Vec::new(),
            warnings: Vec::new(),
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
}

/// A warning emitted by [DestructiveChangesChecker](trait.DestructiveChangesChecker.html). Warnings will
/// prevent a migration from being applied, unless the `force` flag is passed.
#[derive(Debug, Serialize, PartialEq)]
pub struct MigrationWarning {
    pub description: String,
}

/// An error emitted by the [DestructiveChangesChecker](trait.DestructiveChangesChecker.html). Errors will
/// always prevent a migration from being applied.
#[derive(Debug, Serialize, PartialEq)]
pub struct MigrationError {
    pub tpe: String,
    pub description: String,
    pub field: Option<String>,
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

impl<T> DestructiveChangesChecker<T> for EmptyDestructiveChangesChecker<T>
where
    T: Send + Sync + 'static,
{
    fn check(&self, _database_migration: &T) -> ConnectorResult<DestructiveChangeDiagnostics> {
        Ok(DestructiveChangeDiagnostics::new())
    }
}
