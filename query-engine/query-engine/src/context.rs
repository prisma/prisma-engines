use crate::{PrismaError, PrismaResult};
use psl::dml::Datamodel;
use query_core::{executor, schema::QuerySchemaRef, schema_builder, QueryExecutor};
use query_engine_metrics::MetricRegistry;
use std::{env, fmt, sync::Arc};

/// Prisma request context containing all immutable state of the process.
/// There is usually only one context initialized per process.
pub struct PrismaContext {
    /// The api query schema.
    query_schema: QuerySchemaRef,
    /// DML-based v2 datamodel.
    dm: Datamodel,
    pub metrics: MetricRegistry,
    /// Central query executor.
    pub executor: Box<dyn QueryExecutor + Send + Sync + 'static>,
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

    pub async fn build(self) -> PrismaResult<PrismaContext> {
        PrismaContext::new(self.schema, self.enable_raw_queries, self.metrics.unwrap_or_default()).await
    }
}

impl PrismaContext {
    /// Initializes a new Prisma context.
    async fn new(
        schema: psl::ValidatedSchema,
        enable_raw_queries: bool,
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
        let preview_features: Vec<_> = config.preview_features().iter().collect();
        let (db_name, executor) = executor::load(data_source, &preview_features, &url).await?;

        // Build internal data model
        let internal_data_model = prisma_models::convert(&schema, db_name);

        // Construct query schema
        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
            internal_data_model,
            enable_raw_queries,
            data_source.active_connector,
            preview_features,
            data_source.relation_mode(),
        ));

        let context = Self {
            query_schema,
            dm: psl::lift(&schema),
            executor,
            metrics,
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
