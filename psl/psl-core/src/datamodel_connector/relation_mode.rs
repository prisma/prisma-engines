use crate::datamodel_connector::ReferentialAction;
use enumflags2::{bitflags, BitFlags};
use std::fmt;

/// Defines the part of the stack where referential actions are handled.
#[bitflags]
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RelationMode {
    /// Enforced in the database. Needs support from the underlying database
    /// server.
    ForeignKeys,
    /// Enforced in Prisma. Slower, but for databases that do not support
    /// foreign keys.
    Prisma,
    /// Enforced in Prisma. Currently allowed only for MongoDB.
    /// Skips any referential integrity checks (emulated relations) after an update or delete operation.
    /// Using this mode, Prisma will STOP ensuring that every relation-field is valid (left intact), thus stricter checks
    /// must be done at the PrismaClient side before executing a query or with a try-catch pattern.
    PrismaSkipIntegrity,
}

impl RelationMode {
    pub fn allowed_emulated_referential_actions_default() -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        Restrict | SetNull | NoAction | Cascade
    }

    pub fn is_prisma(&self) -> bool {
        matches!(self, Self::Prisma) || matches!(self, Self::PrismaSkipIntegrity)
    }

    pub fn should_skip_emulated_referential_integrity(&self) -> bool {
        self.uses_foreign_keys() || matches!(self, Self::PrismaSkipIntegrity)
    }

    /// True, if integrity is in database foreign keys
    pub fn uses_foreign_keys(&self) -> bool {
        matches!(self, Self::ForeignKeys)
    }
}

impl Default for RelationMode {
    fn default() -> Self {
        Self::ForeignKeys
    }
}

impl fmt::Display for RelationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationMode::ForeignKeys => write!(f, "foreignKeys"),
            RelationMode::Prisma => write!(f, "prisma"),
            RelationMode::PrismaSkipIntegrity => write!(f, "prismaSkipIntegrity"),
        }
    }
}
