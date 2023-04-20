use serde::{Deserialize, Serialize};

use crate::Warning;

/// Defines a view in the database.
#[derive(Debug, Deserialize, Serialize)]
pub struct ViewDefinition {
    /// The database or schema where the view is located.
    pub schema: String,
    /// The name of the view.
    pub name: String,
    /// The database definition of the view.
    pub definition: String,
}

/// The result structure from a successful introspection run.
#[derive(Debug)]
pub struct IntrospectionResult {
    /// Datamodel
    pub data_model: String,
    /// The introspected data model is empty
    pub is_empty: bool,
    /// Introspection warnings
    pub warnings: Vec<Warning>,
    /// Inferred Prisma version
    pub version: Version,
    /// The database view definitions. None if preview feature
    /// is not enabled.
    pub views: Option<Vec<ViewDefinition>>,
}

/// The output type from introspection.
#[derive(Debug, Deserialize, Serialize)]
pub struct IntrospectionResultOutput {
    /// Datamodel
    pub datamodel: String,
    /// warnings
    pub warnings: Vec<Warning>,
    /// version
    pub version: Version,
    /// views
    pub views: Option<Vec<ViewDefinition>>,
}

/// The inferred Prisma version.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Version {
    /// Not a Prisma database.
    NonPrisma,
    /// Maybe a Prisma 1.0 database.
    Prisma1,
    /// Maybe a Prisma 1.1 database.
    Prisma11,
    /// Maybe a Prisma 2 database.
    Prisma2,
}

impl Version {
    /// Is the database a Prisma 1.0 or 1.1 database.
    pub fn is_prisma1(self) -> bool {
        matches!(self, Self::Prisma1 | Self::Prisma11)
    }
}
