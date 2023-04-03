//! Every builder is required to cache input and output type ids that are created inside
//! of them, e.g. as soon as the builder produces a type, it must be retrievable later.
//!
//! The cache has two purposes:
//! - First, break circular dependencies, as they can happen in recursive input / output types.
//! - Second, it serves as a central list of build types of that builder, which are used later to
//!   collect all types of the query schema.

use super::Identifier;
use std::{collections::HashMap, fmt::Debug};

/// HashMap wrapper. Caches keys at most once, and errors on repeated insertion of the same key to
/// uphold schema building consistency guarantees.
#[derive(Debug, Default)]
pub(crate) struct TypeRefCache<T> {
    cache: HashMap<Identifier, T>,
}

impl<T: Copy + Debug> TypeRefCache<T> {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        TypeRefCache {
            cache: HashMap::with_capacity(capacity),
        }
    }

    pub(crate) fn get(&self, ident: &Identifier) -> Option<T> {
        self.cache.get(ident).copied()
    }

    /// Caches given value with given identifier. Panics if the cache key already exists.
    /// The reason is that for the query schema to work, we need identifiers to be valid.
    /// If we insert a new object into the cache that replaces the old one, the reference would be
    /// broken.
    pub(crate) fn insert(&mut self, ident: Identifier, value: T) {
        if let Some(old) = self.cache.insert(ident, value) {
            panic!("Invariant violation: Inserted identifier twice, this is a bug. {old:?}")
        }
    }
}

/// Convenience cache utility to load and return immediately if an input object type is already cached.
macro_rules! return_cached_input {
    ($ctx:expr, $ident:expr) => {
        if let Some(existing_type) = $ctx.get_input_type($ident) {
            return existing_type;
        }
    };
}

/// Convenience cache utility to load and return immediately if an output object type is already cached.
macro_rules! return_cached_output {
    ($ctx:ident, $name:expr) => {
        if let Some(existing_type) = $ctx.get_output_type($name) {
            return existing_type;
        }
    };
}

/// Convenience cache utility to load and return immediately if an output object type is already cached.
macro_rules! return_cached_enum {
    ($ctx:ident, $name:expr) => {
        if let Some(existing_type) = $ctx.get_enum_type($name) {
            return existing_type;
        }
    };
}
