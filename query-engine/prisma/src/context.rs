use crate::{data_model_loader::*, exec_loader, PrismaError, PrismaResult};
use query_core::{
    schema::{QuerySchemaRef, SupportedCapabilities},
    BuildMode, QueryExecutor, QuerySchemaBuilder,
};
// use prisma_models::InternalDataModelRef;
use std::sync::Arc;

/// Prisma request context containing all immutable state of the process.
/// There is usually only one context initialized per process.
pub struct PrismaContext {
    // Internal data model used throughout the query engine.
    //internal_data_model: InternalDataModelRef,
    /// The api query schema.
    query_schema: QuerySchemaRef,

    /// DML-based v2 datamodel.
    dm: datamodel::Datamodel,

    /// Central query executor.
    pub executor: Box<dyn QueryExecutor + Send + Sync + 'static>,
}

pub struct ContextBuilder {
    legacy: bool,
    force_transactions: bool,
    enable_raw_queries: bool,
}

impl ContextBuilder {
    pub fn legacy(mut self, val: bool) -> Self {
        self.legacy = val;
        self
    }

    pub fn force_transactions(mut self, val: bool) -> Self {
        self.force_transactions = val;
        self
    }

    pub fn enable_raw_queries(mut self, val: bool) -> Self {
        self.enable_raw_queries = val;
        self
    }

    pub async fn build(self) -> PrismaResult<PrismaContext> {
        PrismaContext::new(self.legacy, self.force_transactions, self.enable_raw_queries).await
    }
}

impl PrismaContext {
    /// Initializes a new Prisma context.
    /// Loads all immutable state for the query engine:
    /// 1. The data model. This has different options on how to initialize. See data_model_loader module. The Prisma configuration (prisma.yml) is used as fallback.
    /// 2. The data model is converted to the internal data model.
    /// 3. The api query schema is constructed from the internal data model.
    async fn new(legacy: bool, force_transactions: bool, enable_raw_queries: bool) -> PrismaResult<Self> {
        // Load data model in order of precedence.
        let (v2components, template) = load_data_model_components()?;

        let (dm, data_sources) = (v2components.datamodel, v2components.data_sources);

        // We only support one data source at the moment, so take the first one (default not exposed yet).
        let data_source = if data_sources.is_empty() {
            return Err(PrismaError::ConfigurationError("No valid data source found".into()));
        } else {
            data_sources.first().unwrap()
        };

        // Load executor
        let (db_name, executor) = exec_loader::load(&**data_source, force_transactions).await?;

        // Build internal data model
        let internal_data_model = template.build(db_name);

        // Construct query schema
        let build_mode = if legacy { BuildMode::Legacy } else { BuildMode::Modern };
        let capabilities = SupportedCapabilities::empty(); // todo connector capabilities.

        let schema_builder =
            QuerySchemaBuilder::new(&internal_data_model, &capabilities, build_mode, enable_raw_queries);

        let query_schema: QuerySchemaRef = Arc::new(schema_builder.build());

        Ok(Self {
            // internal_data_model,
            query_schema,
            dm,
            executor,
        })
    }

    pub fn builder() -> ContextBuilder {
        ContextBuilder {
            legacy: false,
            force_transactions: false,
            enable_raw_queries: false,
        }
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }

    pub fn datamodel(&self) -> &datamodel::Datamodel {
        &self.dm
    }

    pub fn primary_connector(&self) -> &'static str {
        self.executor.primary_connector()
    }
}
