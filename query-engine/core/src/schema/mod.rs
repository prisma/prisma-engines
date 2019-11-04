#![warn(warnings)]

mod capability;
mod query_schema;
mod renderer;

pub use capability::*;
pub use query_schema::*;
pub use renderer::*;

use std::sync::{Arc, Weak};

/// Since we have the invariant that the weak refs that are used throughout the query
/// schema have to be always valid, we use this simple trait to keep the code clutter low.
pub trait IntoArc<T> {
    fn into_arc(&self) -> Arc<T>;
}

impl<T> IntoArc<T> for Weak<T> {
    fn into_arc(&self) -> Arc<T> {
        self.upgrade().expect("Expected weak reference to be valid.")
    }
}
