use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::future::Either;
use pin_project::pin_project;

use crate::MetricRecorder;

thread_local! {
    static CURRENT_RECORDER: RefCell<Option<MetricRecorder>> = const { RefCell::new(None) };
}

/// Instruments a type with a metrics recorder.
///
/// The instrumentation logic is currently only implemented for futures, but it can be extended to
/// support streams, sinks, and other types in the future when needed.
pub trait WithMetricsInstrumentation: Sized {
    fn with_recorder(self, recorder: MetricRecorder) -> WithRecorder<Self> {
        WithRecorder { inner: self, recorder }
    }

    fn with_optional_recorder(self, recorder: Option<MetricRecorder>) -> Either<WithRecorder<Self>, Self> {
        match recorder {
            Some(recorder) => Either::Left(self.with_recorder(recorder)),
            None => Either::Right(self),
        }
    }

    fn with_current_recorder(self) -> Either<WithRecorder<Self>, Self> {
        CURRENT_RECORDER.with_borrow(|recorder| {
            let recorder = recorder.clone().or_else(crate::recorder::global_recorder);
            self.with_optional_recorder(recorder)
        })
    }
}

impl<T> WithMetricsInstrumentation for T {}

#[pin_project]
pub struct WithRecorder<T> {
    #[pin]
    inner: T,
    recorder: MetricRecorder,
}

impl<T: Future> Future for WithRecorder<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let prev_recorder = CURRENT_RECORDER.replace(Some(this.recorder.clone()));

        let poll = metrics::with_local_recorder(this.recorder, || this.inner.poll(cx));

        CURRENT_RECORDER.set(prev_recorder);

        poll
    }
}
