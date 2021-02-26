mod channel;
mod registry;
mod visitor;

use channel::EventChannel;
use napi::threadsafe_function::ThreadsafeFunction;
use registry::EventRegistry;
use std::future::Future;
use tracing::level_filters::LevelFilter;
use tracing_futures::WithSubscriber;
use tracing_subscriber::layer::{Layered, SubscriberExt};

/// A logger logging to a bounded channel. When in scope, all log messages from
/// the scope are stored to the channel, which must be consumed or after some
/// point, further log lines will just be dropped.
#[derive(Clone)]
pub struct ChannelLogger {
    subscriber: Layered<EventChannel, EventRegistry>,
    level: LevelFilter,
}

impl ChannelLogger {
    /// Creates a new instance of a logger with the minimum log level.
    pub fn new(level: LevelFilter, callback: ThreadsafeFunction<String>) -> Self {
        let mut layer = EventChannel::new(callback);
        layer.filter_level(level);

        Self {
            subscriber: EventRegistry::new().with(layer),
            level,
        }
    }

    /// Wraps a future to a logger, storing all events in the pipeline to
    /// the channel.
    pub async fn with_logging<F, U, T>(&self, f: F) -> crate::Result<T>
    where
        U: Future<Output = crate::Result<T>>,
        F: FnOnce() -> U,
    {
        f().with_subscriber(self.subscriber.clone()).await
    }
}
