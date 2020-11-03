use sql_schema_describer::{
    walkers::{ColumnWalker, SqlSchemaExt, TableWalker},
    SqlSchema,
};

#[derive(Debug)]
pub(crate) struct Pair<T> {
    previous: T,
    next: T,
}

impl<T> Pair<T> {
    pub(crate) fn new(previous: T, next: T) -> Self {
        Pair { previous, next }
    }

    pub(crate) fn previous(&self) -> &T {
        &self.previous
    }

    pub(crate) fn next(&self) -> &T {
        &self.next
    }
}

impl<'a> Pair<&'a SqlSchema> {
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
}
