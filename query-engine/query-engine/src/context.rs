use crate::features::{EnabledFeatures, Feature};
use crate::{logger::Logger, opt::PrismaOpt};
use crate::{PrismaError, PrismaResult};
use psl::PreviewFeature;
use query_core::{executor, protocol::EngineProtocol, schema::QuerySchemaRef, schema_builder, QueryExecutor};
use query_engine_metrics::setup as metric_setup;
use query_engine_metrics::MetricRegistry;
use std::{env, fmt, sync::Arc};
use tracing::Instrument;

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
    /// Enabled features
    pub enabled_features: EnabledFeatures,
}

impl fmt::Debug for PrismaContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrismaContext { .. }")
    }
}

impl PrismaContext {
    pub async fn new(
        schema: psl::ValidatedSchema,
        protocol: EngineProtocol,
        enabled_features: EnabledFeatures,
        metrics: Option<MetricRegistry>,
    ) -> PrismaResult<PrismaContext> {
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
        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
            internal_data_model,
            enabled_features.contains(Feature::RawQueries),
        ));

        let context = Self {
            query_schema,
            executor,
            metrics: metrics.unwrap_or_default(),
            engine_protocol: protocol,
            enabled_features,
        };

        context.verify_connection().await?;

        Ok(context)
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }

    pub fn executor(&self) -> &(dyn QueryExecutor + Send + Sync + 'static) {
        self.executor.as_ref()
    }

    pub fn primary_connector(&self) -> &'static str {
        self.executor.primary_connector().name()
    }

    pub fn engine_protocol(&self) -> EngineProtocol {
        self.engine_protocol
    }

    async fn verify_connection(&self) -> PrismaResult<()> {
        self.executor.primary_connector().get_connection().await?;
        Ok(())
    }
}

pub async fn setup(
    opts: &PrismaOpt,
    install_logger: bool,
    metrics: Option<MetricRegistry>,
) -> PrismaResult<Arc<PrismaContext>> {
    let metrics = metrics.unwrap_or_default();

    if install_logger {
        Logger::new("prisma-engine-http", Some(metrics.clone()), opts)
            .install()
            .unwrap();
    }

    if opts.enable_metrics || opts.dataproxy_metric_override {
        metric_setup();
    }

    let datamodel = opts.schema(false)?;
    let config = &datamodel.configuration;
    let protocol = opts.engine_protocol(config.preview_features());
    config.validate_that_one_datasource_is_provided()?;

    let span = tracing::info_span!("prisma:engine:connect");

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
