use crate::{PrismaError, PrismaResult};
use datamodel::{Configuration, Datamodel};
use prisma_models::DatamodelConverter;
use query_core::{executor, schema::QuerySchemaRef, schema_builder, BuildMode, QueryExecutor};
use std::{env, fmt, sync::Arc};

/// Prisma request context containing all immutable state of the process.
/// There is usually only one context initialized per process.
pub struct PrismaContext {
    /// The api query schema.
    query_schema: QuerySchemaRef,
    /// DML-based v2 datamodel.
    dm: Datamodel,
    /// Central query executor.
    pub executor: Box<dyn QueryExecutor + Send + Sync + 'static>,
}

impl fmt::Debug for PrismaContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrismaContext { .. }")
    }
}

pub struct ContextBuilder {
    legacy: bool,
    enable_raw_queries: bool,
    datamodel: Datamodel,
    config: Configuration,
}

impl ContextBuilder {
    pub fn legacy(mut self, val: bool) -> Self {
        self.legacy = val;
        self
    }

    pub fn enable_raw_queries(mut self, val: bool) -> Self {
        self.enable_raw_queries = val;
        self
    }

    pub async fn build(self) -> PrismaResult<PrismaContext> {
        PrismaContext::new(self.config, self.datamodel, self.legacy, self.enable_raw_queries).await
    }
}

impl PrismaContext {
    /// Initializes a new Prisma context.
    async fn new(config: Configuration, dm: Datamodel, legacy: bool, enable_raw_queries: bool) -> PrismaResult<Self> {
        let template = DatamodelConverter::convert(&dm);

        // We only support one data source at the moment, so take the first one (default not exposed yet).
        let data_source = config
            .datasources
            .first()
            .ok_or_else(|| PrismaError::ConfigurationError("No valid data source found".into()))?;

        let url = data_source.load_url(|key| env::var(key).ok())?;

        // Load executor
        let preview_features: Vec<_> = config.preview_features().cloned().collect();
        let (db_name, executor) = executor::load(&data_source, &preview_features, &url).await?;

        // Build internal data model
        let internal_data_model = template.build(db_name);

        // Construct query schema
        let build_mode = if legacy { BuildMode::Legacy } else { BuildMode::Modern };
        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
            internal_data_model,
            build_mode,
            enable_raw_queries,
            data_source.capabilities(),
            preview_features,
        ));

        let context = Self {
            query_schema,
            dm,
            executor,
        };

        context.verify_connection().await?;

        Ok(context)
    }

    async fn verify_connection(&self) -> PrismaResult<()> {
        self.executor.primary_connector().get_connection().await?;
        Ok(())
    }

    pub fn builder(config: Configuration, datamodel: Datamodel) -> ContextBuilder {
        ContextBuilder {
            legacy: false,
            enable_raw_queries: false,
            datamodel,
            config,
        }
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }

    pub fn datamodel(&self) -> &Datamodel {
        &self.dm
    }

    pub fn primary_connector(&self) -> String {
        self.executor.primary_connector().name()
    }
}
