use std::path::{Path, PathBuf};

use enumflags2::BitFlags;
use psl::{Datasource, PreviewFeature};
use quaint::prelude::SqlFamily;

const INTROSPECTION_FILE_NAME: &str = "introspected.prisma";

/// Input parameters for a database introspection.
pub struct IntrospectionContext {
    /// This should always be true. TODO: change everything where it's
    /// set to false to take the config into account.
    pub render_config: bool,
    /// How many layers of composite types should be traversed on
    /// MongoDB introspection.
    pub composite_type_depth: CompositeTypeDepth,
    previous_schema: psl::ValidatedSchema,
    namespaces: Option<Vec<String>>,
    base_directory_path: PathBuf,
}

impl IntrospectionContext {
    /// Create a new context.
    pub fn new(
        previous_schema: psl::ValidatedSchema,
        composite_type_depth: CompositeTypeDepth,
        namespaces: Option<Vec<String>>,
        base_directory_path: PathBuf,
    ) -> Self {
        IntrospectionContext {
            previous_schema,
            composite_type_depth,
            render_config: true,
            namespaces,
            base_directory_path,
        }
    }

    /// Take the previous schema _but ignore all the datamodel part_, keeping just the
    /// configuration blocks.
    pub fn new_config_only(
        previous_schema: psl::ValidatedSchema,
        composite_type_depth: CompositeTypeDepth,
        namespaces: Option<Vec<String>>,
        base_directory_path: PathBuf,
    ) -> Result<Self, String> {
        let mut config_blocks = String::new();

        for source in previous_schema.db.datasources() {
            config_blocks.push_str(&previous_schema.db.source(source.span.file_id)[source.span.start..source.span.end]);
            config_blocks.push('\n');
        }

        for generator in previous_schema.db.generators() {
            config_blocks
                .push_str(&previous_schema.db.source(generator.span.file_id)[generator.span.start..generator.span.end]);
            config_blocks.push('\n');
        }

        let previous_schema_config_only = psl::parse_schema_multi_without_extensions(&[(
            Self::introspection_file_path_impl(&previous_schema, &base_directory_path).to_string(),
            config_blocks.into(),
        )])?;

        Ok(Self::new(
            previous_schema_config_only,
            composite_type_depth,
            namespaces,
            base_directory_path,
        ))
    }

    /// The PSL file with the previous schema definition.
    pub fn previous_schema(&self) -> &psl::ValidatedSchema {
        &self.previous_schema
    }

    /// The datasource block of the previous PSL file.
    pub fn datasource(&self) -> &Datasource {
        self.previous_schema.configuration.datasources.first().unwrap()
    }

    /// True if relations are enforced with database foreign keys.
    pub fn foreign_keys_enabled(&self) -> bool {
        self.datasource().relation_mode().uses_foreign_keys()
    }

    /// The string source of the PSL schema file.
    pub fn schema_string(&self) -> &str {
        self.previous_schema.db.source_assert_single()
    }

    /// The configuration block of the PSL schema file.
    pub fn configuration(&self) -> &psl::Configuration {
        &self.previous_schema.configuration
    }

    /// The preview features included in the PSL generator block.
    pub fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.previous_schema.configuration.preview_features()
    }

    /// The schemas property in the PSL datasource block.
    pub fn namespaces(&self) -> Option<&[String]> {
        self.namespaces.as_deref()
    }

    /// The SQL family we're using currently.
    pub fn sql_family(&self) -> SqlFamily {
        match self.datasource().active_provider {
            #[cfg(feature = "postgresql")]
            "postgresql" => SqlFamily::Postgres,
            #[cfg(feature = "cockroachdb")]
            "cockroachdb" => SqlFamily::Postgres,
            #[cfg(feature = "sqlite")]
            "sqlite" => SqlFamily::Sqlite,
            #[cfg(feature = "mssql")]
            "sqlserver" => SqlFamily::Mssql,
            #[cfg(feature = "mysql")]
            "mysql" => SqlFamily::Mysql,
            name => unreachable!("The name `{}` for the datamodel connector is not known", name),
        }
    }

    /// Returns the file name into which new introspection data should be written.
    pub fn introspection_file_path(&self) -> std::borrow::Cow<'_, str> {
        Self::introspection_file_path_impl(&self.previous_schema, &self.base_directory_path)
    }

    fn introspection_file_path_impl<'a>(
        previous_schema: &'a psl::ValidatedSchema,
        base_directory_path: &Path,
    ) -> std::borrow::Cow<'a, str> {
        if previous_schema.db.files_count() == 1 {
            let file_id = previous_schema.db.iter_file_ids().next().unwrap();

            previous_schema.db.file_name(file_id).into()
        } else {
            base_directory_path
                .join(INTROSPECTION_FILE_NAME)
                .to_string_lossy()
                .to_string()
                .into()
        }
    }

    /// Removes the rendering of the configuration.
    pub fn without_config_rendering(mut self) -> Self {
        self.render_config = false;
        self
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
            0 => Self::None,
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
