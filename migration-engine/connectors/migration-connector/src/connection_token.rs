use std::{
    hash::Hash,
    sync::{atomic::AtomicUsize, Arc, Once},
};

/// A unique identifier for a database connection.
#[derive(Debug, Clone)]
pub struct ConnectionToken {
    id: usize,
    keep_alive: bool,
    detached: Arc<Once>,
}

impl Eq for ConnectionToken {}

impl PartialEq for ConnectionToken {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Hash for ConnectionToken {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl ConnectionToken {
    /// Generate a new unique connection token.
    pub fn new() -> ConnectionToken {
        // Using an AtomicUsize is fine, because it will always wrap on
        // overflow. You would need to have connections that are usize::MAX
        // apart to be open simultaneously to get a conflict. This can be ruled
        // out in realistic usage.
        static CONNECTION_TOKEN_COUNTER: AtomicUsize = AtomicUsize::new(0);

        let id = CONNECTION_TOKEN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        ConnectionToken {
            id,
            keep_alive: false,
            detached: Arc::new(Once::new()),
        }
    }

    /// Signal that the connection should be kept open and locked until the next request.
    pub fn set_keep_alive(&mut self) {
        self.keep_alive = true;
    }

    /// Returns whether the connection should still be considered in use.
    pub fn is_detached(&self) -> bool {
        self.detached.is_completed()
    }
}

impl Drop for ConnectionToken {
    fn drop(&mut self) {
        if self.keep_alive {
            return;
        }

        self.detached.call_once(|| ())
    }
}
