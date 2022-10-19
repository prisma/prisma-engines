use crate::SqlDatabaseSchema;
use sql_schema_describer::{walkers::Walker, SqlSchema};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Pair<T> {
    pub previous: T,
    pub next: T,
}

impl<T> Pair<T> {
    pub(crate) fn new(previous: T, next: T) -> Self {
        Pair { previous, next }
    }

    pub(crate) fn as_ref(&self) -> Pair<&T> {
        Pair {
            previous: &self.previous,
            next: &self.next,
        }
    }

    /// Map each element to an iterator, and zip the two iterators into an iterator over pairs.
    pub(crate) fn interleave<F, I, O>(&self, f: F) -> impl Iterator<Item = Pair<O>>
    where
        I: IntoIterator<Item = O>,
        F: Fn(&T) -> I,
    {
        f(&self.previous)
            .into_iter()
            .zip(f(&self.next).into_iter())
            .map(Pair::from)
    }

    pub(crate) fn into_tuple(self) -> (T, T) {
        (self.previous, self.next)
    }

    pub(crate) fn map<U>(self, f: impl Fn(T) -> U) -> Pair<U> {
        Pair {
            previous: f(self.previous),
            next: f(self.next),
        }
    }

    pub(crate) fn zip<U>(self, other: Pair<U>) -> Pair<(T, U)> {
        Pair::new((self.previous, other.previous), (self.next, other.next))
    }
}

impl<T> Pair<Option<T>> {
    pub(crate) fn transpose(self) -> Option<Pair<T>> {
        match (self.previous, self.next) {
            (Some(previous), Some(next)) => Some(Pair { previous, next }),
            _ => None,
        }
    }
}

impl<'a> Pair<&'a SqlDatabaseSchema> {
    pub(crate) fn walk<I>(self, ids: Pair<I>) -> Pair<Walker<'a, I>> {
        self.zip(ids).map(|(schema, id)| schema.describer_schema.walk(id))
    }
}

impl<'a> Pair<&'a SqlSchema> {
    pub(crate) fn walk<I>(self, ids: Pair<I>) -> Pair<Walker<'a, I>> {
        self.zip(ids).map(|(schema, id)| schema.walk(id))
    }
}

impl<'a, T> Pair<Walker<'a, T>> {
    pub(crate) fn walk<I>(self, ids: Pair<I>) -> Pair<Walker<'a, I>> {
        self.zip(ids).map(|(w, id)| w.walk(id))
    }
}

impl<T> From<(T, T)> for Pair<T> {
    fn from((previous, next): (T, T)) -> Self {
        Pair { previous, next }
    }
}
