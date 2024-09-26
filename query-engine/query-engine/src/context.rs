use crate::features::{EnabledFeatures, Feature};
use crate::{logger::Logger, opt::PrismaOpt};
use crate::{PrismaError, PrismaResult};
use psl::PreviewFeature;
use query_core::{
    protocol::EngineProtocol,
    relation_load_strategy,
    schema::{self, QuerySchemaRef},
    QueryExecutor,
};
use query_engine_metrics::setup as metric_setup;
use query_engine_metrics::MetricRegistry;
use request_handlers::{load_executor, ConnectorKind};
use std::{env, fmt, sync::Arc};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Prisma request context containing all immutable state of the process.
/// There is usually only one context initialized per process.
pub struct PrismaContext {
    /// The api query schema.
    query_schema: QuerySchemaRef,
    /// The metrics registry
    pub(crate) metrics: MetricRegistry,
    /// Central query executor.
    pub(crate) executor: Box<dyn QueryExecutor + Send + Sync + 'static>,
    /// The engine protocol in use
    pub(crate) engine_protocol: EngineProtocol,
    /// Enabled features
    pub(crate) enabled_features: EnabledFeatures,
}

impl fmt::Debug for PrismaContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrismaContext { .. }")
    }
}

impl PrismaContext {
    pub(crate) async fn new(
        schema: psl::ValidatedSchema,
        protocol: EngineProtocol,
        enabled_features: EnabledFeatures,
        metrics: Option<MetricRegistry>,
    ) -> PrismaResult<PrismaContext> {
        let arced_schema = Arc::new(schema);
        let arced_schema_2 = Arc::clone(&arced_schema);

        let query_schema_fut = tokio::runtime::Handle::current().spawn_blocking(move || {
            // Construct query schema
            schema::build(arced_schema, enabled_features.contains(Feature::RawQueries))
        });
        let executor_fut = tokio::spawn(async move {
            let config = &arced_schema_2.configuration;
            let preview_features = config.preview_features();

            // We only support one data source at the moment, so take the first one (default not exposed yet).
            let datasource = config
                .datasources
                .first()
                .ok_or_else(|| PrismaError::ConfigurationError("No valid data source found".into()))?;

            let url = datasource.load_url(|key| env::var(key).ok())?;
            // Load executor
            let executor = load_executor(ConnectorKind::Rust { url, datasource }, preview_features).await?;
            let conn = executor.primary_connector().get_connection().await?;
            let db_version = conn.version().await;

            PrismaResult::<_>::Ok((executor, db_version))
        });

        let (query_schema, executor_with_db_version) = tokio::join!(query_schema_fut, executor_fut);
        let (executor, db_version) = executor_with_db_version.unwrap()?;

        let query_schema = query_schema.unwrap().with_db_version_supports_join_strategy(
            relation_load_strategy::db_version_supports_joins_strategy(db_version)?,
        );

        let context = Self {
            query_schema: Arc::new(query_schema),
            executor,
            metrics: metrics.unwrap_or_default(),
            engine_protocol: protocol,
            enabled_features,
        };

        Ok(context)
    }

    pub(crate) fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }

    pub(crate) fn executor(&self) -> &(dyn QueryExecutor + Send + Sync + 'static) {
        self.executor.as_ref()
    }

    pub(crate) fn primary_connector(&self) -> &'static str {
        self.executor.primary_connector().name()
    }

    pub(crate) fn engine_protocol(&self) -> EngineProtocol {
        self.engine_protocol
    }
}

pub async fn setup(opts: &PrismaOpt, metrics: Option<MetricRegistry>) -> PrismaResult<Arc<PrismaContext>> {
    let metrics = metrics.unwrap_or_default();

    Logger::new("prisma-engine-http", Some(metrics.clone()), opts)
        .install()
        .unwrap();

    if opts.enable_metrics || opts.dataproxy_metric_override {
        metric_setup();
    }

    let datamodel = opts.schema(false)?;
    let config = &datamodel.configuration;
    let protocol = opts.engine_protocol();
    config.validate_that_one_datasource_is_provided()?;

    let span = tracing::info_span!("prisma:engine:connect", user_facing = true);
    if let Some(trace_context) = opts.trace_context.as_ref() {
        let parent_context = telemetry::helpers::restore_remote_context_from_json_str(trace_context);
        span.set_parent(parent_context);
    }

    let mut features = EnabledFeatures::from(opts);

    if config.preview_features().contains(PreviewFeature::Metrics) || opts.dataproxy_metric_override {
        features |= Feature::Metrics
    }

    let cx = PrismaContext::new(datamodel, protocol, features, Some(metrics))
        .instrument(span)
        .await?;

    let state = Arc::new(cx);
    Ok(state)
}
