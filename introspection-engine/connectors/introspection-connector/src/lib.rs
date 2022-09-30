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

    async fn introspect(&self, ctx: &IntrospectionContext) -> ConnectorResult<IntrospectionResult>;
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
    pub code: u32,
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
    pub previous_data_model: Datamodel,
    pub source: Datasource,
    pub composite_type_depth: CompositeTypeDepth,
    pub preview_features: BitFlags<PreviewFeature>,
    previous_schema: psl::ValidatedSchema,
}

impl IntrospectionContext {
    pub fn new(previous_schema: psl::ValidatedSchema, composite_type_depth: CompositeTypeDepth) -> Self {
        let mut ctx = Self::new_naive(previous_schema, composite_type_depth);
        ctx.previous_data_model = psl::lift(&ctx.previous_schema);
        ctx
    }

    /// Take the previous schema _but ignore all the datamodel part_, keeping just the
    /// configuration blocks.
    pub fn new_config_only(previous_schema: psl::ValidatedSchema, composite_type_depth: CompositeTypeDepth) -> Self {
        let mut config_blocks = String::new();

        for source in previous_schema.db.ast().sources() {
            config_blocks.push_str(&previous_schema.db.source()[source.span.start..source.span.end]);
            config_blocks.push('\n');
        }

        for generator in previous_schema.db.ast().generators() {
            config_blocks.push_str(&previous_schema.db.source()[generator.span.start..generator.span.end]);
            config_blocks.push('\n');
        }

        let previous_schema_config_only = psl::parse_schema(config_blocks).unwrap();

        Self::new_naive(previous_schema_config_only, composite_type_depth)
    }

    fn new_naive(previous_schema: psl::ValidatedSchema, composite_type_depth: CompositeTypeDepth) -> Self {
        let source = previous_schema.configuration.datasources.clone().pop().unwrap();
        let preview_features = previous_schema.configuration.preview_features();

        IntrospectionContext {
            previous_data_model: psl::dml::Datamodel::new(),
            previous_schema,
            source,
            composite_type_depth,
            preview_features,
        }
    }

    pub fn foreign_keys_enabled(&self) -> bool {
        self.source.relation_mode().uses_foreign_keys()
    }

    pub fn schema_string(&self) -> &str {
        self.previous_schema.db.source()
    }

    pub fn configuration(&self) -> &psl::Configuration {
        &self.previous_schema.configuration
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
