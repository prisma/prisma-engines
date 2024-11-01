/// When the `metrics` feature is disabled, we don't compile the `prisma-metrics` crate and
/// thus can't use the metrics instrumentation. To avoid the boilerplate of putting every
/// `with_current_recorder` call behind `#[cfg]`, we use this stub trait that does nothing but
/// allows the code that relies on `WithMetricsInstrumentation` trait to be in scope compile.
#[cfg(not(feature = "metrics"))]
pub(crate) trait MetricsInstrumentationStub: Sized {
    fn with_current_recorder(self) -> Self {
        self
    }
}

#[cfg(not(feature = "metrics"))]
impl<T> MetricsInstrumentationStub for T {}
