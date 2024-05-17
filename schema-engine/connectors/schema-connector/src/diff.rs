use crate::migrations_directory::MigrationDirectory;
use psl::parser_database::SourceFile;
use std::fmt::Debug;

/// Diffable things
pub enum DiffTarget<'a> {
    /// A Prisma schema.
    Datamodel(Vec<(String, SourceFile)>),
    /// A migrations folder. What is diffable is the state of the database schema at the end of the
    /// migrations history.
    Migrations(&'a [MigrationDirectory]),
    /// A live database connection string.
    Database,
    /// Assume an empty database schema.
    Empty,
}

impl Debug for DiffTarget<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffTarget::Datamodel(_) => f.debug_struct("DiffTarget::Datamodel").finish(),
            DiffTarget::Migrations(_) => f.debug_struct("DiffTarget::Migrations").finish(),
            DiffTarget::Database => f.debug_struct("DiffTarget::Database").finish(),
            DiffTarget::Empty => f.debug_struct("DiffTarget::Empty").finish(),
        }
    }
}
