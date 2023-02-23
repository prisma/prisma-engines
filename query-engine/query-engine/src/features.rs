use crate::opt::PrismaOpt;

/// Feature models the engine feature toggles that can be set up during the startup via options
/// See [PrismaOpt]'s boolean flags, for each of the corresponding feature toggles.
#[enumflags2::bitflags]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Feature {
    DataProxyMetricOverride,
    DebugMode,
    LogQueries,
    Metrics,
    OpenTelemetry,
    Playground,
    RawQueries,
    TelemetryInResponse,
}

/// EnabledFeatures represents a collection of the engine features that are enabled, masked in a byte
pub type EnabledFeatures = enumflags2::BitFlags<Feature>;

impl From<&PrismaOpt> for EnabledFeatures {
    fn from(opts: &PrismaOpt) -> Self {
        let mut features: EnabledFeatures = Self::default();

        if opts.dataproxy_metric_override {
            features |= Feature::DataProxyMetricOverride
        }
        if opts.enable_debug_mode {
            features |= Feature::DebugMode
        }
        if opts.log_queries {
            features |= Feature::LogQueries
        }
        if opts.enable_metrics {
            features |= Feature::Metrics
        }
        if opts.enable_open_telemetry {
            features |= Feature::OpenTelemetry
        }
        if opts.enable_playground {
            features |= Feature::Playground
        }
        if opts.enable_raw_queries {
            features |= Feature::RawQueries
        }
        if opts.enable_telemetry_in_response {
            features |= Feature::TelemetryInResponse
        }

        features
    }
}
