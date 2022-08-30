mod error;

pub use error::{ConnectorError, ErrorKind};

use enumflags2::BitFlags;
use psl::{common::preview_features::PreviewFeature, dml::Datamodel, Datasource};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type ConnectorResult<T> = Result<T, ConnectorError>;

#[async_trait::async_trait]
pub trait IntrospectionConnector: Send + Sync + 'static {
    async fn list_databases(&self) -> ConnectorResult<Vec<String>>;

    async fn get_metadata(&self) -> ConnectorResult<DatabaseMetadata>;

    async fn get_database_description(&self) -> ConnectorResult<String>;

    async fn get_database_version(&self) -> ConnectorResult<String>;

    async fn introspect(
        &self,
        existing_data_model: &Datamodel,
        ctx: IntrospectionContext,
    ) -> ConnectorResult<IntrospectionResult>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseMetadata {
    pub table_count: usize,
    pub size_in_bytes: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Version {
    NonPrisma,
    Prisma1,
    Prisma11,
    Prisma2,
}

#[derive(Debug)]
pub struct IntrospectionResult {
    /// Datamodel
    pub data_model: Datamodel,
    /// Introspection warnings
    pub warnings: Vec<Warning>,
    /// Inferred Prisma version
    pub version: Version,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Warning {
    pub code: i16,
    pub message: String,
    pub affected: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IntrospectionResultOutput {
    /// Datamodel
    pub datamodel: String,
    /// warnings
    pub warnings: Vec<Warning>,
    /// version
    pub version: Version,
}

pub struct IntrospectionContext {
    pub source: Datasource,
    pub composite_type_depth: CompositeTypeDepth,
    pub preview_features: BitFlags<PreviewFeature>,
}

impl IntrospectionContext {
    pub fn foreign_keys_enabled(&self) -> bool {
        self.source.referential_integrity().uses_foreign_keys()
    }
}

/// Control type for composite type traversal.
#[derive(Debug, Clone, Copy)]
pub enum CompositeTypeDepth {
    /// Allow maximum of n layers of nested types.
    Level(usize),
    /// Unrestricted traversal.
    Infinite,
    /// No traversal, typing into dynamic Json.
    None,
}

impl From<isize> for CompositeTypeDepth {
    fn from(size: isize) -> Self {
        match size {
            size if size < 0 => Self::Infinite,
            size if size == 0 => Self::None,
            _ => Self::Level(size as usize),
        }
    }
}

impl Default for CompositeTypeDepth {
    fn default() -> Self {
        Self::None
    }
}

impl CompositeTypeDepth {
    /// Traversal is not allowed.
    pub fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// Go one level down in nested composite types.
    pub fn level_down(self) -> CompositeTypeDepth {
        match self {
            CompositeTypeDepth::Level(level) if level > 1 => Self::Level(level - 1),
            CompositeTypeDepth::Level(_) => Self::None,
            CompositeTypeDepth::Infinite => Self::Infinite,
            CompositeTypeDepth::None => Self::None,
        }
    }
}
