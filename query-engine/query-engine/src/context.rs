use crate::{exec_loader, PrismaError, PrismaResult};
use query_core::{
    schema::{QuerySchemaRef, SupportedCapabilities},
    BuildMode, QueryExecutor, QuerySchemaBuilder,
};
// use prisma_models::InternalDataModelRef;
use datamodel::{Configuration, Datamodel};
use prisma_models::DatamodelConverter;
use std::sync::Arc;

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

        // Load executor
        let (db_name, executor) = exec_loader::load(&data_source).await?;

        // Build internal data model
        let internal_data_model = template.build(db_name);

        // Construct query schema
        let build_mode = if legacy { BuildMode::Legacy } else { BuildMode::Modern };

        let capabilities = SupportedCapabilities::empty(); // todo connector capabilities.

        let schema_builder =
            QuerySchemaBuilder::new(&internal_data_model, &capabilities, build_mode, enable_raw_queries);

        let query_schema: QuerySchemaRef = Arc::new(schema_builder.build());

        Ok(Self {
            query_schema,
            dm,
            executor,
        })
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

    pub fn primary_connector(&self) -> &'static str {
        self.executor.primary_connector()
    }
}
