#[cfg(not(feature = "metrics"))]
pub(crate) trait MetricsInstrumentationStub: Sized {
    fn with_current_recorder(self) -> Self {
        self
    }
}

#[cfg(not(feature = "metrics"))]
impl<T> MetricsInstrumentationStub for T {}
