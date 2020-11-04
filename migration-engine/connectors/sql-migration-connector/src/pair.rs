use sql_schema_describer::{
    walkers::{ColumnWalker, EnumWalker, IndexWalker, SqlSchemaExt, TableWalker},
    SqlSchema,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Pair<T> {
    previous: T,
    next: T,
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

    pub(crate) fn as_tuple(&self) -> (&T, &T) {
        (&self.previous, &self.next)
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

    pub(crate) fn previous(&self) -> &T {
        &self.previous
    }

    pub(crate) fn next(&self) -> &T {
        &self.next
    }
}

impl<'a> Pair<&'a SqlSchema> {
    pub(crate) fn enums(&self, enum_indexes: &Pair<usize>) -> Pair<EnumWalker<'a>> {
        Pair::new(
            self.previous().enum_walker_at(enum_indexes.previous),
            self.next.enum_walker_at(enum_indexes.next),
        )
    }

    pub(crate) fn tables(&self, table_indexes: &Pair<usize>) -> Pair<TableWalker<'a>> {
        Pair::new(
            self.previous().table_walker_at(*table_indexes.previous()),
            self.next.table_walker_at(*table_indexes.next()),
        )
    }
}

impl<'a> Pair<TableWalker<'a>> {
    pub(crate) fn columns(&self, column_indexes: &Pair<usize>) -> Pair<ColumnWalker<'a>> {
        Pair::new(
            self.previous().column_at(*column_indexes.previous()),
            self.next().column_at(*column_indexes.next()),
        )
    }

    pub(crate) fn indexes(&self, index_indexes: &Pair<usize>) -> Pair<IndexWalker<'a>> {
        self.as_ref().zip(index_indexes.as_ref()).map(|(t, i)| t.index_at(*i))
    }
}
