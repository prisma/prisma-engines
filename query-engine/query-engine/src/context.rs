use crate::{PrismaError, PrismaResult};
use query_core::{executor, protocol::EngineProtocol, schema::QuerySchemaRef, schema_builder, QueryExecutor};
use query_engine_metrics::MetricRegistry;
use std::{env, fmt, sync::Arc};

/// Prisma request context containing all immutable state of the process.
/// There is usually only one context initialized per process.
pub struct PrismaContext {
    /// The api query schema.
    query_schema: QuerySchemaRef,
    /// The metrics registry
    pub metrics: MetricRegistry,
    /// Central query executor.
    pub executor: Box<dyn QueryExecutor + Send + Sync + 'static>,
    /// The engine protocol in use
    pub engine_protocol: EngineProtocol,
    /// Server configuration
    pub server_config: Option<ServerConfig>,
}

impl fmt::Debug for PrismaContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrismaContext { .. }")
    }
}
#[derive(Default, Copy, Clone)]
pub struct ServerConfig {
    pub enable_playground: bool,
    pub enable_debug_mode: bool,
    pub enable_metrics: bool,
}

pub struct ContextBuilder {
    enable_raw_queries: bool,
    schema: psl::ValidatedSchema,
    metrics: Option<MetricRegistry>,
    protocol: EngineProtocol,
    engine_flags: ServerConfig,
}

impl ContextBuilder {
    pub fn enable_raw_queries(mut self, val: bool) -> Self {
        self.enable_raw_queries = val;
        self
    }

    pub fn set_metrics(mut self, metrics: MetricRegistry) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn set_engine_flags(mut self, enable_playground: bool, enable_debug_mode: bool, enable_metrics: bool) -> Self {
        self.engine_flags = ServerConfig {
            enable_debug_mode,
            enable_metrics,
            enable_playground,
        };
        self
    }

    pub async fn build(self) -> PrismaResult<PrismaContext> {
        PrismaContext::new(
            self.schema,
            self.enable_raw_queries,
            self.protocol,
            self.metrics.unwrap_or_default(),
        )
        .await
    }
}

impl PrismaContext {
    /// Initializes a new Prisma context.
    async fn new(
        schema: psl::ValidatedSchema,
        enable_raw_queries: bool,
        protocol: EngineProtocol,
        metrics: MetricRegistry,
    ) -> PrismaResult<Self> {
        let config = &schema.configuration;
        // We only support one data source at the moment, so take the first one (default not exposed yet).
        let data_source = config
            .datasources
            .first()
            .ok_or_else(|| PrismaError::ConfigurationError("No valid data source found".into()))?;

        let url = data_source.load_url(|key| env::var(key).ok())?;

        // Load executor
        let executor = executor::load(data_source, config.preview_features(), &url).await?;

        // Build internal data model
        let internal_data_model = prisma_models::convert(Arc::new(schema));

        // Construct query schema
        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(internal_data_model, enable_raw_queries));

        let context = Self {
            query_schema,
            executor,
            metrics,
            engine_protocol: protocol,
            server_config: Default::default(),
        };

        context.verify_connection().await?;

        Ok(context)
    }

    async fn verify_connection(&self) -> PrismaResult<()> {
        self.executor.primary_connector().get_connection().await?;
        Ok(())
    }

    pub fn builder(schema: psl::ValidatedSchema, protocol: EngineProtocol) -> ContextBuilder {
        ContextBuilder {
            enable_raw_queries: false,
            schema,
            metrics: None,
            protocol,
            engine_flags: Default::default(),
        }
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }

    pub fn executor(&self) -> &(dyn QueryExecutor + Send + Sync + 'static) {
        &*self.executor
    }

    pub fn primary_connector(&self) -> &'static str {
        self.executor.primary_connector().name()
    }

    pub fn engine_protocol(&self) -> EngineProtocol {
        self.engine_protocol
    }
}
