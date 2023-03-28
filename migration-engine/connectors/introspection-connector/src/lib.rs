mod error;

pub use error::{ConnectorError, ErrorKind};

use enumflags2::BitFlags;
use psl::{Datasource, PreviewFeature};
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Version {
    NonPrisma,
    Prisma1,
    Prisma11,
    Prisma2,
}

impl Version {
    pub fn is_prisma1(self) -> bool {
        matches!(self, Self::Prisma1 | Self::Prisma11)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ViewDefinition {
    /// The database or schema where the view is located.
    pub schema: String,
    /// The name of the view.
    pub name: String,
    /// The database definition of the view.
    pub definition: String,
}

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
    pub views: Option<Vec<ViewDefinition>>,
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
    /// views
    pub views: Option<Vec<ViewDefinition>>,
}

pub struct IntrospectionContext {
    /// This should always be true. TODO: change everything where it's
    /// set to false to take the config into account.
    pub render_config: bool,
    pub composite_type_depth: CompositeTypeDepth,
    previous_schema: psl::ValidatedSchema,
    namespaces: Option<Vec<String>>,
}

impl IntrospectionContext {
    pub fn new(
        previous_schema: psl::ValidatedSchema,
        composite_type_depth: CompositeTypeDepth,
        namespaces: Option<Vec<String>>,
    ) -> Self {
        IntrospectionContext {
            previous_schema,
            composite_type_depth,
            render_config: true,
            namespaces,
        }
    }

    /// Take the previous schema _but ignore all the datamodel part_, keeping just the
    /// configuration blocks.
    pub fn new_config_only(
        previous_schema: psl::ValidatedSchema,
        composite_type_depth: CompositeTypeDepth,
        namespaces: Option<Vec<String>>,
    ) -> Self {
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

        Self::new(previous_schema_config_only, composite_type_depth, namespaces)
    }

    pub fn previous_schema(&self) -> &psl::ValidatedSchema {
        &self.previous_schema
    }

    pub fn datasource(&self) -> &Datasource {
        self.previous_schema.configuration.datasources.first().unwrap()
    }

    pub fn foreign_keys_enabled(&self) -> bool {
        self.datasource().relation_mode().uses_foreign_keys()
    }

    pub fn schema_string(&self) -> &str {
        self.previous_schema.db.source()
    }

    pub fn configuration(&self) -> &psl::Configuration {
        &self.previous_schema.configuration
    }

    pub fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.previous_schema.configuration.preview_features()
    }

    pub fn namespaces(&self) -> Option<&[String]> {
        self.namespaces.as_deref()
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
