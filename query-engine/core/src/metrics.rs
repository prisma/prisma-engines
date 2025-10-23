/// Stub trait for metrics instrumentation. This trait allows code that previously relied
/// on `WithMetricsInstrumentation` trait to compile without metrics support.
pub(crate) trait MetricsInstrumentationStub: Sized {
    fn with_current_recorder(self) -> Self {
        self
    }
}

impl<T> MetricsInstrumentationStub for T {}
