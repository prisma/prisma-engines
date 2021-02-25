mod channel;
mod registry;
mod visitor;

use std::{future::Future, sync::Arc};

use channel::EventChannel;
use registry::EventRegistry;
use tokio::sync::{mpsc, Mutex};
use tracing::level_filters::LevelFilter;
use tracing_futures::WithSubscriber;
use tracing_subscriber::layer::{Layered, SubscriberExt};

/// A logger logging to a bounded channel. When in scope, all log messages from
/// the scope are stored to the channel, which must be consumed or after some
/// point, further log lines will just be dropped.
#[derive(Clone)]
pub struct ChannelLogger {
    receiver: Arc<Mutex<mpsc::Receiver<String>>>,
    subscriber: Layered<EventChannel, EventRegistry>,
    level: LevelFilter,
}

impl ChannelLogger {
    /// Creates a new instance of a logger with the minimum log level.
    pub fn new(level: LevelFilter) -> Self {
        // We store 10_000 events, after that we drop until we have
        // space again. We might want this to be user-configurable
        // in the future.
        let (sender, receiver) = mpsc::channel(10000);

        let mut layer = EventChannel::new(sender);
        layer.filter_level(level);

        Self {
            receiver: Arc::new(Mutex::new(receiver)),
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

    /// Takes the oldest log event from the channel. None, if channel is
    /// closed.
    pub async fn next_event(&self) -> Option<String> {
        self.receiver.lock().await.recv().await
    }

    /// A special event to notify the JavaScript listener to stop listening,
    /// helping us to get around of the problem of not having streams on
    /// JavaScript.
    pub async fn disconnect_listeners(&self) -> crate::Result<()> {
        self.with_logging(|| async {
            tracing::info!("disconnected");
            Ok(())
        })
        .await
    }
}
