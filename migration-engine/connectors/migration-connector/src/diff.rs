use crate::migrations_directory::MigrationDirectory;
use std::{borrow::Cow, fmt::Debug};

/// Diffable things
pub enum DiffTarget<'a> {
    /// A Prisma schema
    Datamodel(Cow<'a, str>),
    /// A migrations folder. What is diffable is the state of the database schema at the end of the
    /// migrations history.
    Migrations(Cow<'a, [MigrationDirectory]>),
    /// A live database connection string.
    Database(Cow<'a, str>),
    /// Assume an empty database schema
    Empty,
}

impl Debug for DiffTarget<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffTarget::Datamodel(_) => f.debug_struct("DiffTarget::Datamodel").finish(),
            DiffTarget::Migrations(_) => f.debug_struct("DiffTarget::Migrations").finish(),
            DiffTarget::Database(_) => f.debug_struct("DiffTarget::Database").finish(),
            DiffTarget::Empty => f.debug_struct("DiffTarget::Empty").finish(),
        }
    }
}

impl DiffTarget<'_> {
    /// Try interpreting the DiffTarget as a Datamodel variant.
    pub fn as_datamodel(&self) -> Option<&str> {
        match self {
            DiffTarget::Datamodel(schema) => Some(schema),
            _ => None,
        }
    }
}
