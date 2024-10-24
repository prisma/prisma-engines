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
    /// The current metric recorder temporarily set on the current thread while polling a future.
    ///
    /// See the description of `GLOBAL_RECORDER` in [`crate::recorder`] module for more
    /// information.
    static CURRENT_RECORDER: RefCell<Option<MetricRecorder>> = const { RefCell::new(None) };
}

/// Instruments a type with a metrics recorder.
///
/// The instrumentation logic is currently only implemented for futures, but it could be extended
/// to support streams, sinks, and other types later if needed. Right now we only need it to be
/// able to set the initial recorder in the Node-API engine methods and forward the recorder to
/// spawned tokio tasks; in other words, to instrument the top-level future of each task.
pub trait WithMetricsInstrumentation: Sized {
    /// Instruments the type with a [`MetricRecorder`].
    fn with_recorder(self, recorder: MetricRecorder) -> WithRecorder<Self> {
        WithRecorder { inner: self, recorder }
    }

    /// Instruments the type with an [`MetricRecorder`] if it is a `Some` or returns `self` as is
    /// if the `recorder` is a `None`.
    fn with_optional_recorder(self, recorder: Option<MetricRecorder>) -> Either<WithRecorder<Self>, Self> {
        match recorder {
            Some(recorder) => Either::Left(self.with_recorder(recorder)),
            None => Either::Right(self),
        }
    }

    /// Instruments the type with the current [`MetricRecorder`] from the parent context on this
    /// thread, or the default global recorder otherwise. If neither is set, then `self` is
    /// returned as is.
    fn with_current_recorder(self) -> Either<WithRecorder<Self>, Self> {
        CURRENT_RECORDER.with_borrow(|recorder| {
            let recorder = recorder.clone().or_else(crate::recorder::global_recorder);
            self.with_optional_recorder(recorder)
        })
    }
}

impl<T> WithMetricsInstrumentation for T {}

/// A type instrumented with a metric recorder.
///
/// If `T` is a `Future`, then `WithRecorder<T>` is also a `Future`. When polled, it temporarily
/// sets the local metric recorder for the duration of polling the inner future, and then restores
/// the previous recorder on the stack.
///
/// Similar logic can be implemented for cases where `T` is another async primitive like a stream
/// or a sink, or any other type where such instrumentation makes sense (e.g. function).
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
