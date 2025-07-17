use crate::SqlDatabaseSchema;
use sql_schema_describer::{SqlSchema, walkers::Walker};

/// A pair of items that can exist in two schemas: previous is the item in the previous / old
/// schema, next is the item in the next / new schema.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MigrationPair<T> {
    pub previous: T,
    pub next: T,
}

impl<T> MigrationPair<T> {
    pub(crate) fn new(previous: T, next: T) -> Self {
        MigrationPair { previous, next }
    }

    pub(crate) fn as_ref(&self) -> MigrationPair<&T> {
        MigrationPair {
            previous: &self.previous,
            next: &self.next,
        }
    }

    /// Map each element to an iterator, and zip the two iterators into an iterator over pairs.
    pub(crate) fn interleave<F, I, O>(&self, f: F) -> impl Iterator<Item = MigrationPair<O>>
    where
        I: IntoIterator<Item = O>,
        F: Fn(&T) -> I,
    {
        f(&self.previous)
            .into_iter()
            .zip(f(&self.next))
            .map(MigrationPair::from)
    }

    pub(crate) fn into_tuple(self) -> (T, T) {
        (self.previous, self.next)
    }

    pub(crate) fn map<U>(self, f: impl Fn(T) -> U) -> MigrationPair<U> {
        MigrationPair {
            previous: f(self.previous),
            next: f(self.next),
        }
    }

    pub(crate) fn zip<U>(self, other: MigrationPair<U>) -> MigrationPair<(T, U)> {
        MigrationPair::new((self.previous, other.previous), (self.next, other.next))
    }
}

impl<T> MigrationPair<Option<T>> {
    pub(crate) fn transpose(self) -> Option<MigrationPair<T>> {
        match (self.previous, self.next) {
            (Some(previous), Some(next)) => Some(MigrationPair { previous, next }),
            _ => None,
        }
    }
}

impl<'a> MigrationPair<&'a SqlDatabaseSchema> {
    pub(crate) fn walk<I>(self, ids: MigrationPair<I>) -> MigrationPair<Walker<'a, I>> {
        self.zip(ids).map(|(schema, id)| schema.describer_schema.walk(id))
    }
}

impl<'a> MigrationPair<&'a SqlSchema> {
    pub(crate) fn walk<I>(self, ids: MigrationPair<I>) -> MigrationPair<Walker<'a, I>> {
        self.zip(ids).map(|(schema, id)| schema.walk(id))
    }
}

impl<'a, T> MigrationPair<Walker<'a, T>> {
    pub(crate) fn walk<I>(self, ids: MigrationPair<I>) -> MigrationPair<Walker<'a, I>> {
        self.zip(ids).map(|(w, id)| w.walk(id))
    }
}

impl<T> From<(T, T)> for MigrationPair<T> {
    fn from((previous, next): (T, T)) -> Self {
        MigrationPair { previous, next }
    }
}
