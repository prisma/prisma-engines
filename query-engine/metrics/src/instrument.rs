use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::MetricRecorder;

thread_local! {
    static CURRENT_RECORDER: RefCell<Option<Arc<MetricRecorder>>> = const { RefCell::new(None) };
}

pub trait MetricsFutureExt: Future {
    fn with_recorder(self, recorder: Arc<MetricRecorder>) -> WithRecorder<Self::Output>;
    fn with_current_recorder(self) -> WithRecorder<Self::Output>;
}

impl<F> MetricsFutureExt for F
where
    F: Future + Send + 'static,
{
    fn with_recorder(self, recorder: Arc<MetricRecorder>) -> WithRecorder<F::Output> {
        WithRecorder {
            inner: Box::pin(self),
            recorder,
        }
    }

    fn with_current_recorder(self) -> WithRecorder<Self::Output> {
        CURRENT_RECORDER.with_borrow(|recorder| {
            let recorder = recorder
                .clone()
                .or_else(crate::recorder::global_recorder)
                .expect("with_current_recorder called outside of a future instrumented using with_recorder or without global recoder");
            self.with_recorder(recorder)
        })
    }
}

pub struct WithRecorder<T> {
    inner: Pin<Box<dyn Future<Output = T> + Send>>,
    recorder: Arc<MetricRecorder>,
}

impl<T> Future for WithRecorder<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let recorder = Arc::clone(&self.recorder);
        let prev_recorder = CURRENT_RECORDER.replace(Some(Arc::clone(&recorder)));

        let poll = metrics::with_local_recorder(&*recorder, || self.inner.as_mut().poll(cx));

        CURRENT_RECORDER.set(prev_recorder);

        poll
    }
}
