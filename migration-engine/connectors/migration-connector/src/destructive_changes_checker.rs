use crate::ConnectorResult;
use std::marker::PhantomData;

pub trait DestructiveChangesChecker<T>: Send + Sync + 'static
where
    T: 'static,
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

#[derive(Debug, Serialize, PartialEq)]
pub struct MigrationWarning {
    pub description: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct MigrationError {
    pub tpe: String,
    pub description: String,
    pub field: Option<String>,
}

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
