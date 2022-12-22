use crate::{capture_tracer::CaptureExporter, PrismaError, PrismaResult};
use query_core::{executor, schema::QuerySchemaRef, schema_builder, QueryExecutor};
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
    // The trace capturer being in flight.
    pub trace_capturer: Option<CaptureExporter>,
}

impl fmt::Debug for PrismaContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrismaContext { .. }")
    }
}

pub struct ContextBuilder {
    enable_raw_queries: bool,
    schema: psl::ValidatedSchema,
    metrics: Option<MetricRegistry>,
    trace_capturer: Option<CaptureExporter>,
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

    pub fn set_trace_capturer(mut self, trace_capturer: Option<CaptureExporter>) -> Self {
        self.trace_capturer = trace_capturer;
        self
    }

    pub async fn build(self) -> PrismaResult<PrismaContext> {
        PrismaContext::new(
            self.schema,
            self.enable_raw_queries,
            self.metrics.unwrap_or_default(),
            self.trace_capturer,
        )
        .await
    }
}

impl PrismaContext {
    /// Initializes a new Prisma context.
    pub async fn new(
        schema: psl::ValidatedSchema,
        enable_raw_queries: bool,
        metrics: MetricRegistry,
        trace_capturer: Option<CaptureExporter>,
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
            trace_capturer,
        };

        context.verify_connection().await?;

        Ok(context)
    }

    async fn verify_connection(&self) -> PrismaResult<()> {
        self.executor.primary_connector().get_connection().await?;
        Ok(())
    }

    pub fn builder(schema: psl::ValidatedSchema) -> ContextBuilder {
        ContextBuilder {
            enable_raw_queries: false,
            schema,
            metrics: None,
            trace_capturer: None,
        }
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }

    pub fn primary_connector(&self) -> &'static str {
        self.executor.primary_connector().name()
    }
}
