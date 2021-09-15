use std::fmt;

use dml::relation_info::ReferentialAction;
use enumflags2::{bitflags, BitFlags};

/// Defines the part of the stack where referential actions are handled.
#[bitflags]
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReferentialIntegrity {
    /// Enforced in the database. Needs support from the underlying database
    /// server.
    ForeignKeys,
    /// Enforced in Prisma. Slower, but for databases that do not support
    /// foreign keys.
    Prisma,
}

impl ReferentialIntegrity {
    /// Returns either the given actions if foreign keys are used, or the
    /// allowed emulated actions if referential integrity happens in Prisma.
    pub fn allowed_referential_actions(
        &self,
        from_connector: BitFlags<ReferentialAction>,
    ) -> BitFlags<ReferentialAction> {
        use ReferentialAction::*;

        match self {
            Self::ForeignKeys => from_connector,
            // The emulated modes should be listed here.
            Self::Prisma => Restrict | SetNull | NoAction | Cascade,
        }
    }

    /// True, if integrity is in database foreign keys
    pub fn uses_foreign_keys(&self) -> bool {
        matches!(self, Self::ForeignKeys)
    }
}

impl Default for ReferentialIntegrity {
    fn default() -> Self {
        Self::ForeignKeys
    }
}

impl fmt::Display for ReferentialIntegrity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferentialIntegrity::ForeignKeys => write!(f, "foreignKeys"),
            ReferentialIntegrity::Prisma => write!(f, "prisma"),
        }
    }
}
